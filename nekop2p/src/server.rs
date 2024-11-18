use std::{net::SocketAddr, sync::Arc};

use dashmap::{DashMap, DashSet};
use delay_map::HashSetDelay;
use tarpc::{client, context::Context, serde_transport::tcp, tokio_serde::formats::Bincode};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{Indexer, IndexerClient, PeerClient};

/// Reference [Indexer] implementation
#[derive(Clone)]
pub struct IndexerServer {
    /// Address of the remote peer
    addr: SocketAddr,

    /// Index shared between all connections
    index: Arc<DashMap<String, DashSet<SocketAddr>>>,

    /// Index shared between all connections to map remote peers with their incoming download port
    dl_ports: Arc<DashMap<SocketAddr, u16>>,

    /// List of neighboring superpeers
    neighbors: Arc<Vec<SocketAddr>>,

    /// Log of all seen query msg_ids
    backtrace: Arc<RwLock<HashSetDelay<Uuid>>>,
}

impl IndexerServer {
    /// Create a new [IndexerServer] with a shared `index` and `dl_ports` for `addr`
    pub fn new(
        addr: SocketAddr,
        index: &Arc<DashMap<String, DashSet<SocketAddr>>>,
        dl_ports: &Arc<DashMap<SocketAddr, u16>>,
        neighbors: &Arc<Vec<SocketAddr>>,
        backtrace: &Arc<RwLock<HashSetDelay<Uuid>>>,
    ) -> Self {
        IndexerServer {
            addr,
            index: Arc::clone(index),
            dl_ports: Arc::clone(dl_ports),
            neighbors: Arc::clone(neighbors),
            backtrace: Arc::clone(backtrace),
        }
    }

    /// Prints all entries in index
    pub fn print_index(self) {
        self.index.iter().for_each(|entry| {
            let filename = entry.key();
            entry.value().iter().for_each(|v| {
                let peer = v.key();
                println!("{filename}: {peer}");
            });
        });
    }
}

impl Indexer for IndexerServer {
    async fn set_port(self, _: Context, dl_port: u16) {
        self.dl_ports.insert(self.addr, dl_port);
    }

    async fn register(self, _: Context, filename: String) {
        println!("Registered {filename} for {0}", self.addr);
        {
            let list = self.index.entry(filename).or_default();
            list.insert(self.addr);
        }
        self.print_index();
    }

    async fn search(self, _: Context, filename: String) -> Vec<SocketAddr> {
        println!("Searched {filename} for {0}", self.addr);
        self.index
            .entry(filename)
            .or_default()
            .iter()
            .filter_map(|e| match self.dl_ports.get(&e) {
                Some(x) => {
                    let mut n = e.clone();
                    n.set_port(*x);
                    Some(n)
                }
                None => None,
            })
            .collect()
    }

    async fn deregister(self, _: Context, filename: String) {
        println!("Deregistered {filename} for {0}", self.addr);
        {
            let list = self.index.entry(filename).or_default();
            list.remove(&self.addr);
        }
        self.print_index();
    }

    async fn disconnect_peer(self, _: Context) {
        println!("Clean-up peer {0}", self.addr);

        // scrub index of ip
        self.index.iter().for_each(|entry| {
            entry.value().remove(&self.addr);
        });

        // remove saved port
        self.dl_ports.remove(&self.addr);

        self.print_index();
    }

    async fn query(self, c: Context, msg_id: Uuid, filename: String, ttl: u8) -> Vec<SocketAddr> {
        println!("Querying {filename} for {0} (id: {msg_id})", self.addr);
        // if msg_id has already been seen, then we ignore the query
        if self.backtrace.read().await.contains_key(&msg_id) {
            println!("Message {msg_id} already handled!");
            return Vec::new();
        }

        // insert into set of seen msg_ids
        self.backtrace.write().await.insert(msg_id);

        // get peers from this peer's index
        println!("Searched {filename} for {0}", self.addr);
        let mut peers: Vec<_> = self
            .index
            .entry(filename.clone())
            .or_default()
            .iter()
            .filter_map(|e| match self.dl_ports.get(&e) {
                Some(x) => {
                    let mut n = e.clone();
                    n.set_port(*x);
                    Some(n)
                }
                None => None,
            })
            .collect();

        // propogate query to neighboring peers
        if ttl > 0 {
            for peer in self.neighbors.iter() {
                println!(
                    "Propagating query of {filename} to {0} (id: {msg_id})",
                    peer
                );
                if let Ok(transport) = tcp::connect(peer, Bincode::default).await {
                    let client = IndexerClient::new(client::Config::default(), transport).spawn();
                    peers.append(
                        &mut client
                            .query(c, msg_id, filename.clone(), ttl - 1)
                            .await
                            .unwrap_or_default(),
                    );
                }
            }
        }

        peers
    }

    async fn invalidate(
        self,
        c: Context,
        msg_id: Uuid,
        origin_server: SocketAddr,
        filename: String,
    ) {
        println!(
            "Invalidation message for {filename}::{0} sent by {1} (id: {msg_id})",
            origin_server, self.addr
        );
        // if msg_id has already been seen, then we ignore the query
        if self.backtrace.read().await.contains_key(&msg_id) {
            println!("Message {msg_id} already handled!");
            return;
        }

        // insert into set of seen msg_ids
        self.backtrace.write().await.insert(msg_id);

        // send invalidation message to leaf nodes
        println!("Searched {filename} for {0}", self.addr);
        for peer in self
            .index
            .entry(filename.clone())
            .or_default()
            .iter()
            .filter_map(|e| match self.dl_ports.get(&e) {
                Some(x) => {
                    let mut n = e.clone();
                    n.set_port(*x);
                    Some(n)
                }
                None => None,
            })
            .into_iter()
        {
            if peer == origin_server {
                // skip original leaf node
                continue;
            }
            println!(
                "Propagating invalidation of {filename} to {0} (id: {msg_id})",
                peer
            );
            if let Ok(transport) = tcp::connect(peer, Bincode::default).await {
                let client = PeerClient::new(client::Config::default(), transport).spawn();
                let _ = client
                    .invalidate(c, msg_id, origin_server, filename.clone())
                    .await;
            }
        }

        // invalidate all leaf nodes that weren't the origin server
        self.index
            .entry(filename.clone())
            .or_default()
            .retain(|e| match self.dl_ports.get(&e) {
                Some(x) => {
                    let mut n = e.clone();
                    n.set_port(*x);
                    n == origin_server
                }
                None => false,
            });

        // propogate invalidation to neighboring indexers
        for peer in self.neighbors.iter() {
            println!(
                "Propagating query of {filename} to {0} (id: {msg_id})",
                peer
            );
            if let Ok(transport) = tcp::connect(peer, Bincode::default).await {
                let client = IndexerClient::new(client::Config::default(), transport).spawn();
                let _ = client
                    .invalidate(c, msg_id, origin_server, filename.clone())
                    .await;
            }
        }
    }
}

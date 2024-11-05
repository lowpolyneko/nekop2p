use std::{net::SocketAddr, sync::Arc};

use dashmap::{DashMap, DashSet};
use tarpc::{client, context::Context, serde_transport::tcp, tokio_serde::formats::Bincode};
use uuid::Uuid;

use crate::{Indexer, IndexerClient};

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
    backtrace: Arc<DashSet<Uuid>>,
}

impl IndexerServer {
    /// Create a new [IndexerServer] with a shared `index` and `dl_ports` for `addr`
    pub fn new(
        addr: SocketAddr,
        index: &Arc<DashMap<String, DashSet<SocketAddr>>>,
        dl_ports: &Arc<DashMap<SocketAddr, u16>>,
        neighbors: &Arc<Vec<SocketAddr>>,
        backtrace: &Arc<DashSet<Uuid>>,
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
        println!("Queried {filename} for {0}", self.addr);
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
        // if msg_id has already been seen, then we ignore the query
        if self.backtrace.contains(&msg_id) {
            return Vec::new();
        }

        // get peers from this peer's index
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
                let transport = tcp::connect(peer, Bincode::default).await.unwrap();
                let client = IndexerClient::new(client::Config::default(), transport).spawn();
                peers.append(
                    &mut client
                        .query(c, msg_id, filename.clone(), ttl - 1)
                        .await
                        .unwrap_or_default(),
                );
            }
        }

        peers
    }
}

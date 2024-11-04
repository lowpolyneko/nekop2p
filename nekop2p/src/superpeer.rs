use std::net::SocketAddr;
use std::sync::Arc;

use dashmap::{DashMap, DashSet};
use serde::Deserialize;
use tarpc::{client, context::Context, serde_transport::tcp, tokio_serde::formats::Bincode};
use tokio::fs;
use uuid::Uuid;

use crate::{SuperPeer, SuperPeerClient};

pub struct TTLEntry<T> {
    val: T,
    ttl: u64,
}

impl<T> TTLEntry<T> {
    /// Timed unwrapped
    fn timed_unwrap(&self) -> Option<&T> {
        if unix_time() > self.ttl {
            Some(&self.val)
        } else {
            None
        }
    }
}

/// [SuperPeer] config values
#[derive(Deserialize)]
pub struct SuperPeerConfig {
    /// Host
    pub host: Option<String>,

    /// [SuperPeerServer] Port
    pub port: u16,

    /// Node type specific config, either [SuperNodeConfig] or [LeafNodeConfig]
    pub node_config: NodeConfig,
}

#[derive(Deserialize)]
pub enum NodeConfig {
    SuperNode(SuperNodeConfig),
    LeafNode(LeafNodeConfig),
}

#[derive(Deserialize)]
pub struct SuperNodeConfig {
    /// List of leaf nodes
    pub leaf_nodes: Vec<SocketAddr>,

    /// List of leaf nodes
    pub superpeer_neighbors: Vec<SocketAddr>,
}

#[derive(Deserialize)]
pub struct LeafNodeConfig {
    /// [SuperPeer] to connect to
    pub superpeer: SocketAddr,
}

/// Reference [SuperPeer] overlay implementation
#[derive(Clone)]
pub struct SuperPeerServer {
    /// Address of the remote peer
    addr: SocketAddr,

    /// Config for super peer
    config: Arc<SuperPeerConfig>,

    /// Index shared between all connections
    index: Option<Arc<DashMap<String, DashSet<SocketAddr>>>>,

    /// Index for back-propogation of queries
    backtrace: Arc<DashMap<Uuid, TTLEntry<SocketAddr>>>,
}

impl SuperPeerServer {
    pub fn new(
        addr: SocketAddr,
        config: &Arc<SuperPeerConfig>,
        index: Option<&Arc<DashMap<String, DashSet<SocketAddr>>>>,
        backtrace: &Arc<DashMap<Uuid, TTLEntry<SocketAddr>>>,
    ) -> Self {
        SuperPeerServer {
            addr,
            config: Arc::clone(config),
            index: match index {
                Some(i) => Some(Arc::clone(i)),
                None => None,
            },
            backtrace: Arc::clone(backtrace),
        }
    }

    /// Prune all expired entries in backtrace
    fn prune_backtrace_table(self) {}

    /// Prints all entries in index
    fn print_index(self) {
        if let Some(index) = self.index {
            index.iter().for_each(|entry| {
                let filename = entry.key();
                entry.value().iter().for_each(|v| {
                    let peer = v.key();
                    println!("{filename}: {peer}");
                });
            });
        }
    }
}

impl SuperPeer for SuperPeerServer {
    async fn query(self, c: Context, msg_id: Uuid, ttl: u8, filename: String) {
        if self.backtrace.contains_key(&msg_id) {
            // already exists, meaning query has been processed. ignore.
            return;
        }

        // on query, append to backtrace table
        // TODO don't hardcode TTL
        self.backtrace.insert(
            msg_id,
            TTLEntry {
                val: self.addr,
                ttl: unix_time() + 30,
            },
        );

        // check for files, returning a [query_hit] on success
        if let Some(index) = self.index {
            println!("Queried {filename} for {0}", self.addr);
            index
                .entry(filename)
                .or_default()
                .iter()
                .for_each(|peer| async {
                    let transport = tcp::connect(peer, Bincode::default);
                    let client =
                        SuperPeerClient::new(client::Config::default(), transport.await.unwrap())
                            .spawn();
                    let _ = client.query_hit(c, msg_id, ttl, 
                });
        }

        // propogate query if ttl is non-zero
        if ttl < 1 {
            return;
        }

        if let NodeConfig::SuperNode(node_config) = &self.config.node_config {
            for ip in &node_config.leaf_nodes {
                let transport = tcp::connect(ip, Bincode::default);
                let client =
                    SuperPeerClient::new(client::Config::default(), transport.await.unwrap())
                        .spawn();
                let _ = client.query(c, msg_id, ttl - 1, filename.clone()).await;
            }
        }
    }

    async fn query_hit(
        self,
        _: Context,
        msg_id: Uuid,
        ttl: u8,
        filename: String,
        peer: SocketAddr,
    ) {
        // TODO handle non-existant msg id
        let back_addr = match self.backtrace.get(&msg_id).unwrap().timed_unwrap() {
            Some(x) => x,
            None => return,
        };
    }

    async fn obtain(self, _: Context, filename: String) -> Option<Vec<u8>> {
        println!(
            "Handling download request for {0} from {1}",
            filename, self.addr
        );
        fs::read(filename).await.ok()
    }

    async fn register(self, _: Context, filename: String, dl_port: u16) {
        println!("Registered {filename} for {0}", self.addr);
        if let Some(index) = &self.index {
            let list = index.entry(filename).or_default();
            let mut addr = self.addr;
            addr.set_port(dl_port);
            list.insert(addr);
        }
        self.print_index();
    }

    async fn deregister(self, _: Context, filename: String, dl_port: u16) {
        println!("Deregistered {filename} for {0}", self.addr);
        if let Some(index) = &self.index {
            let list = index.entry(filename).or_default();
            let mut addr = self.addr;
            addr.set_port(dl_port);
            list.remove(&addr);
        }
        self.print_index();
    }
}

fn unix_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

use std::net::SocketAddr;
use std::sync::Arc;

use dashmap::DashMap;
use serde::Deserialize;
use tarpc::context::Context;
use uuid::Uuid;

use crate::SuperPeer;

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

    /// List of leaf nodes
    pub leaf_nodes: Vec<SocketAddr>,

    /// List of leaf nodes
    pub superpeer_neighbors: Vec<SocketAddr>,
}

/// Reference [SuperPeer] overlay implementation
#[derive(Clone)]
pub struct SuperPeerServer {
    /// Address of the remote peer
    addr: SocketAddr,

    /// Config for super peer
    config: Arc<SuperPeerConfig>,

    /// Index for back-propogation of queries
    backtrace: Arc<DashMap<Uuid, TTLEntry<SocketAddr>>>,
}

impl SuperPeerServer {
    pub fn new(addr: SocketAddr, config: &Arc<SuperPeerConfig>, backtrace: &Arc<DashMap<Uuid, TTLEntry<SocketAddr>>>) -> Self {
        SuperPeerServer {
            addr,
            config: Arc::clone(config),
            backtrace: Arc::clone(backtrace),
        }
    }

    fn prune_backtrace_table(self) {}
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
        self.obtain(c, filename).await;

        // propogate query if ttl is non-zero
        if ttl < 1 {
            return;
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
        None
    }
}

fn unix_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

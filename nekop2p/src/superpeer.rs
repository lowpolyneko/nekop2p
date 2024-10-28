use std::{net::SocketAddr, ops::Deref};
use std::sync::Arc;

use dashmap::DashMap;
use tarpc::context::Context;
use uuid::Uuid;

use crate::{SuperPeer, IndexerServer};

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

/// Reference [SuperPeer] overlay implementation
#[derive(Clone)]
pub struct SuperPeerServer {
    /// Underlying [IndexerServer]
    s: IndexerServer,

    /// Index for back-propogation of queries
    backtrace: Arc<DashMap<Uuid, TTLEntry<SocketAddr>>>,
}

impl SuperPeerServer {
    pub fn new(s: IndexerServer, backtrace: &Arc<DashMap<Uuid, TTLEntry<SocketAddr>>>) -> Self {
        SuperPeerServer {
            s,
            backtrace: Arc::clone(backtrace),
        }
    }

    fn prune_backtrace_table(self) {}
}

impl SuperPeer for SuperPeerServer {
    async fn query(self, _: Context, msg_id: Uuid, ttl: u8, filename: String) {
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
}

impl Deref for SuperPeerServer {
    type Target = IndexerServer;
    fn deref(&self) -> &Self::Target {
        &self.s
    }
}

fn unix_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

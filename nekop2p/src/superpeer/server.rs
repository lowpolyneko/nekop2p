use std::ops::Deref;
use std::net::SocketAddr;

use tarpc::context::Context;

use crate::{IndexerServer, SuperPeer};

struct TTLEntry<T> {
    val: T,
    ttl: i64,
}

/// Reference [crate::Indexer] implementation with [SuperPeer] support
#[derive(Clone)]
pub struct SuperIndexerServer {
    /// Underlying [IndexerServer]
    s: IndexerServer,
}

impl SuperPeer for SuperIndexerServer {
    fn query(self, _: Context, msg_id: String, ttl: u8, filename: String) {

    }

    fn query_hit(self, _: Context, msg_id: String, ttl: u8, filename: String, peer: SocketAddr) {

    }
}

impl Deref for SuperIndexerServer {
    type Target = IndexerServer;
    fn deref(&self) -> &Self::Target {
        &self.s
    }
}

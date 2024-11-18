//! Common library for nekop2p
//!
//! Contains the schemas for RPC between peers and for the indexer. Service traits are defined as
//! [Peer] and [Indexer].
//!
//! Both a peer and indexer reference server are provided in [PeerServer] and [IndexerServer]
//! respectively.
//!
//! Clients are utilized using [tarpc]'s generated [PeerClient] and [IndexerClient].
mod peer;
mod server;
pub use peer::{Metadata, PeerServer};
pub use server::IndexerServer;

use std::net::SocketAddr;

use uuid::Uuid;

/// RPC scheme for interacting with an [IndexerServer]
#[tarpc::service]
pub trait Indexer {
    /// Map the IP address of this peer to `dl_port` as the corrisponding incoming port to connect
    /// to if another peer wishes to download from this peer.
    async fn set_port(dl_port: u16);

    /// Register `filename` in index
    async fn register(filename: String);

    /// Query `filename` in index and returns a [Vec] of [SocketAddr] with the
    /// connection details for all peers which have `filename`
    async fn search(filename: String) -> Vec<SocketAddr>;

    /// Deregister `filename` in index
    async fn deregister(filename: String);

    /// Remove all mentions of peer from index and dl_ports
    async fn disconnect_peer();

    /// Queries entire network for `filename` with a given ttl
    async fn query(msg_id: Uuid, filename: String, ttl: u8) -> Vec<SocketAddr>;

    /// Spreads an invalidation message across the network for `filename` owned by `origin_server`
    /// (Peer endpoint)
    async fn invalidate(msg_id: Uuid, origin_server: SocketAddr, filename: String);
}

/// RPC scheme for interacting with a [PeerServer]
#[tarpc::service]
pub trait Peer {
    /// Query `filename` and send over the raw bytes (and a ttr) if it exists
    async fn download_file(filename: String) -> Option<(Vec<u8>, Metadata)>;

    /// Invalidates a `filename` on endpoint, discard if version number is older
    async fn invalidate(msg_id: Uuid, origin_server: SocketAddr, filename: String);
}

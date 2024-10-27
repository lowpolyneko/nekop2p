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
pub mod superpeer;
pub use peer::PeerServer;
pub use server::IndexerServer;

use std::net::SocketAddr;

/// RPC scheme for querying neighboring [IndexerServer]s or [PeerServer]s
#[tarpc::service]
pub trait SuperPeer {
    /// Query the network for `filename`
    async fn query(msg_id: String, ttl: u8, filename: String);

    /// Inform a peer that a given peer has `filename` via back propogation
    async fn query_hit(msg_id: String, ttl: u8, filename: String, peer: SocketAddr);
}

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
}

/// RPC scheme for interacting with a [PeerServer]
#[tarpc::service]
pub trait Peer {
    /// Query `filename` and send over the raw bytes if it exists
    async fn download_file(filename: String) -> Option<Vec<u8>>;
}

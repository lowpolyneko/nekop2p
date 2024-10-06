pub mod server;

use std::net::SocketAddr;

#[tarpc::service]
pub trait Indexer {
    async fn set_port(dl_port: u16);
    async fn register(filename: String);
    async fn search(filename: String) -> Vec<SocketAddr>;
    async fn deregister(filename: String);
    async fn disconnect_peer();
}

#[tarpc::service]
pub trait Peer {
    async fn download_file(filename: String) -> Option<Vec<u8>>;
}

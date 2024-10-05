use std::net::SocketAddr;

#[tarpc::service]
pub trait Indexer {
    async fn register(filename: String);
    async fn search(filename: String) -> Vec<SocketAddr>;
    async fn deregister(filename: String);
}

#[tarpc::service]
pub trait Peer {
    async fn download_file(filename: String) -> Vec<u8>;
}

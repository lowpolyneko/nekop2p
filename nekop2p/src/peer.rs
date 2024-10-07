use std::net::SocketAddr;

use tarpc::context::Context;
use tokio::fs;

use crate::Peer;

/// Reference [Peer] implementation
#[derive(Clone)]
pub struct PeerServer {
    /// Address of remote peer
    addr: SocketAddr,
}

impl PeerServer {
    /// Create a new [PeerServer] with the address of the remote peer
    pub fn new(addr: SocketAddr) -> Self {
        PeerServer { addr }
    }
}

impl Peer for PeerServer {
    async fn download_file(self, _: Context, filename: String) -> Option<Vec<u8>> {
        println!(
            "Handling download request for {0} from {1}",
            filename, self.addr
        );
        fs::read(filename).await.ok()
    }
}

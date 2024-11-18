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

    async fn invalidate(
        self,
        _: Context,
        _: uuid::Uuid,
        origin_server: SocketAddr,
        filename: String,
        _: u8,
    ) {
        // got an invalidation message of a file, assume file is bad and delete
        println!(
            "Recieved invalidation message for {0}::{1} from {2}",
            filename, origin_server, self.addr
        );
        let _ = fs::remove_file(filename).await;
    }
}

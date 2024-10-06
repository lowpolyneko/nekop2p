use std::net::SocketAddr;

use tarpc::context::Context;
use tokio::fs;

use nekop2p::Peer;

#[derive(Clone)]
pub struct PeerServer {
    addr: SocketAddr,
}

impl PeerServer {
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

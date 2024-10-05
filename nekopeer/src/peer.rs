use nekop2p::Peer;
use tarpc::context::Context;
use tokio::fs;

#[derive(Clone)]
pub struct PeerServer;

impl Peer for PeerServer {
    async fn download_file(self, _: Context, filename: String) -> Vec<u8> {
        fs::read(filename).await.unwrap_or_default()
    }
}

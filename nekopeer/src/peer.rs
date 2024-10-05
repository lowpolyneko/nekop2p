use nekop2p::Peer;
use tarpc::context::Context;

#[derive(Clone)]
pub struct PeerServer;

impl Peer for PeerServer {
    async fn download_file(self, context: Context, filename: String) -> Vec<u8> {
        vec![]
    }
}

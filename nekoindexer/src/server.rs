use tarpc::context::Context;

use nekop2p::Indexer;

#[derive(Clone)]
pub struct IndexerServer;

impl Indexer for IndexerServer {
    async fn register(self, _: Context, filename: String) {
        format!("{filename}");
    }
    async fn search(self, _: Context, filename: String) {
        format!("{filename}");
    }
    async fn deregister(self, _: Context, filename: String) {
        format!("{filename}");
    }
}

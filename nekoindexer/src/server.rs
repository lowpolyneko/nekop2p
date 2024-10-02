use std::collections::{HashMap, HashSet};

use tarpc::context::Context;

use nekop2p::Indexer;

#[derive(Clone)]
pub struct IndexerServer {
    index: HashMap<String, HashSet<String>>,
}

impl IndexerServer {
    pub fn new() -> Self {
        IndexerServer { index: HashMap::new() }
    }
}

impl Indexer for IndexerServer {
    async fn register(mut self, c: Context, filename: String) {
        println!("Registered {filename}");
        let list = self.index.entry(filename).or_default();
        list.insert(c.trace_context.span_id.to_string());
    }
    async fn search(mut self, _: Context, filename: String) -> Vec<String> {
        println!("Queried {filename}");
        self.index.entry(filename).or_default().iter().cloned().collect()
    }
    async fn deregister(mut self, c: Context, filename: String) {
        println!("Deregistered {filename}");
        let list = self.index.entry(filename).or_default();

        list.remove(&c.trace_context.span_id.to_string());
    }
}

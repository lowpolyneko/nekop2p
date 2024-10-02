use std::sync::Arc;

use dashmap::{DashMap, DashSet};
use tarpc::context::Context;

use nekop2p::Indexer;

#[derive(Clone)]
pub struct IndexerServer {
    index: Arc<DashMap<String, DashSet<String>>>,
}

impl IndexerServer {
    pub fn new(index: &Arc<DashMap<String, DashSet<String>>>) -> Self {
        IndexerServer {
            index: Arc::clone(index)
        }
    }

    pub fn print_index(self) {
        self.index.iter().for_each(|entry| {
            let filename = entry.key();
            entry.value().iter().for_each(|v| {
                let peer = v.key();
                println!("{filename}: {peer}");
            });
        });
    }
}

impl Indexer for IndexerServer {
    async fn register(self, c: Context, filename: String) {
        println!("Registered {filename}");
        {
            let list = self.index.entry(filename).or_default();
            list.insert(c.trace_context.span_id.to_string());
        }
        self.print_index();
    }

    async fn search(self, _: Context, filename: String) -> Vec<String> {
        println!("Queried {filename}");
        self.index
            .entry(filename)
            .or_default()
            .iter()
            .map(|e| e.key().clone())
            .collect()
    }

    async fn deregister(self, c: Context, filename: String) {
        println!("Deregistered {filename}");
        {
            let list = self.index.entry(filename).or_default();
            list.remove(&c.trace_context.span_id.to_string());
        }
        self.print_index();
    }
}

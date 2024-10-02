use std::collections::{HashMap, HashSet};

use tarpc::context::Context;

use nekop2p::Indexer;

#[derive(Clone)]
pub struct IndexerServer {
    index: HashMap<String, HashSet<String>>,
}

impl IndexerServer {
    pub fn new() -> Self {
        IndexerServer {
            index: HashMap::new(),
        }
    }

    pub fn print_index(self) {
        for (key, val) in self.index.iter() {
            for v in val.iter() {
                println!("{key}: {v}");
            }
        }
    }
}

impl Indexer for IndexerServer {
    async fn register(mut self, c: Context, filename: String) {
        println!("Registered {filename}");
        let list = self.index.entry(filename).or_default();
        list.insert(c.trace_context.span_id.to_string());
        self.print_index();
    }
    async fn search(mut self, _: Context, filename: String) -> Vec<String> {
        println!("Queried {filename}");
        self.index
            .entry(filename)
            .or_default()
            .iter()
            .cloned()
            .collect()
    }
    async fn deregister(mut self, c: Context, filename: String) {
        println!("Deregistered {filename}");
        let list = self.index.entry(filename).or_default();
        list.remove(&c.trace_context.span_id.to_string());
        self.print_index();
    }
}

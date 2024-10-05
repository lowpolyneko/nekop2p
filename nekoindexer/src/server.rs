use std::{net::SocketAddr, sync::Arc};

use dashmap::{DashMap, DashSet};
use tarpc::context::Context;

use nekop2p::Indexer;

#[derive(Clone)]
pub struct IndexerServer {
    addr: SocketAddr,
    index: Arc<DashMap<String, DashSet<SocketAddr>>>,
}

impl IndexerServer {
    pub fn new(addr: SocketAddr, index: &Arc<DashMap<String, DashSet<SocketAddr>>>) -> Self {
        IndexerServer {
            addr,
            index: Arc::clone(index),
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
    async fn register(self, _: Context, filename: String) {
        println!("Registered {filename}");
        {
            let list = self.index.entry(filename).or_default();
            list.insert(self.addr);
        }
        self.print_index();
    }

    async fn search(self, _: Context, filename: String) -> Vec<SocketAddr> {
        println!("Queried {filename}");
        self.index
            .entry(filename)
            .or_default()
            .iter()
            .map(|e| e.key().clone())
            .collect()
    }

    async fn deregister(self, _: Context, filename: String) {
        println!("Deregistered {filename}");
        {
            let list = self.index.entry(filename).or_default();
            list.remove(&self.addr);
        }
        self.print_index();
    }
}

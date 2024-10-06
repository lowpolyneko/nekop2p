use std::{net::SocketAddr, sync::Arc};

use dashmap::{DashMap, DashSet};
use tarpc::context::Context;

use nekop2p::Indexer;

#[derive(Clone)]
pub struct IndexerServer {
    addr: SocketAddr,
    index: Arc<DashMap<String, DashSet<SocketAddr>>>,
    dl_ports: Arc<DashMap<SocketAddr, u16>>,
}

impl IndexerServer {
    pub fn new(
        addr: SocketAddr,
        index: &Arc<DashMap<String, DashSet<SocketAddr>>>,
        dl_ports: &Arc<DashMap<SocketAddr, u16>>,
    ) -> Self {
        IndexerServer {
            addr,
            index: Arc::clone(index),
            dl_ports: Arc::clone(dl_ports),
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
    async fn set_port(self, _: Context, dl_port: u16) {
        self.dl_ports.insert(self.addr, dl_port);
    }

    async fn register(self, _: Context, filename: String) {
        println!("Registered {filename} for {0}", self.addr);
        {
            let list = self.index.entry(filename).or_default();
            list.insert(self.addr);
        }
        self.print_index();
    }

    async fn search(self, _: Context, filename: String) -> Vec<SocketAddr> {
        println!("Queried {filename} for {0}", self.addr);
        self.index
            .entry(filename)
            .or_default()
            .iter()
            .filter_map(|e| match self.dl_ports.get(&e) {
                Some(x) => {
                    let mut n = e.clone();
                    n.set_port(*x);
                    Some(n)
                }
                None => None,
            })
            .collect()
    }

    async fn deregister(self, _: Context, filename: String) {
        println!("Deregistered {filename} for {0}", self.addr);
        {
            let list = self.index.entry(filename).or_default();
            list.remove(&self.addr);
        }
        self.print_index();
    }

    async fn disconnect_peer(self, _: Context) {
        println!("Clean-up peer {0}", self.addr);

        // scrub index of ip
        self.index.iter().for_each(|entry| {
            entry.value().remove(&self.addr);
        });

        // remove saved port
        self.dl_ports.remove(&self.addr);

        self.print_index();
    }
}

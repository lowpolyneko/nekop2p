use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use dashmap::DashMap;
use futures::{future, prelude::*};
use tarpc::{
    serde_transport::tcp,
    server::{BaseChannel, Channel},
    tokio_serde::formats::Bincode,
};

use nekop2p::Indexer;

mod server;
use crate::server::IndexerServer;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    // host
    host: Option<String>,

    // port
    #[arg(short, long, default_value_t = 5000)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let host = args.host.unwrap_or("0.0.0.0".to_owned());

    println!("Starting indexer on {0}:{1}", host, args.port);

    let index = Arc::new(DashMap::new());
    let dl_ports = Arc::new(DashMap::new());
    let listener = tcp::listen((host, args.port), Bincode::default).await?;
    listener
        // Ignore accept errors.
        .filter_map(|r| future::ready(r.ok()))
        // Establish serve channel
        .map(BaseChannel::with_defaults)
        .map(|channel| {
            let server =
                IndexerServer::new(channel.transport().peer_addr().unwrap(), &index, &dl_ports);
            channel
                .execute(server.serve())
                .for_each(|response| async move {
                    tokio::spawn(response);
                })
        })
        // Max 10 channels.
        .buffer_unordered(10)
        .for_each(|_| async {})
        .await;

    Ok(())
}

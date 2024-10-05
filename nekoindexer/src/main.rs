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
    host: String,

    // port
    #[arg(short, long, default_value_t = 5000)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("Starting server on {0}:{1}", args.host, args.port);

    let index = Arc::new(DashMap::new());
    let listener = tcp::listen((args.host, args.port), Bincode::default).await?;
    listener
        // Ignore accept errors.
        .filter_map(|r| future::ready(r.ok()))
        // Establish serve channel
        .map(BaseChannel::with_defaults)
        .map(|channel| {
            let server = IndexerServer::new(channel.transport().peer_addr().unwrap(), &index);
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

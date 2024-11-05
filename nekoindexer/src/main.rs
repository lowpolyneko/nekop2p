//! Simple binary wrapping the reference implementation of [IndexerServer] in a
//! [tarpc::serde_transport::tcp::connect].
//!
//! Utilizes two [DashMap]s as the underlying data structure for the [IndexerServer::index] and
//! [IndexerServer::dl_ports].
use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use dashmap::{DashMap, DashSet};
use futures::{future, prelude::*};
use tarpc::{
    serde_transport::tcp,
    server::{BaseChannel, Channel},
    tokio_serde::formats::Bincode,
};

use nekop2p::{Indexer, IndexerServer};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    host: Option<String>,

    #[arg(short, long, default_value_t = 5000)]
    port: u16,
}

/// Starts an [IndexerServer] on [Args::host] with [Args::port]
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let host = args.host.unwrap_or("localhost".to_owned());

    println!("Starting indexer on {0}:{1}", host, args.port);

    let index = Arc::new(DashMap::new());
    let dl_ports = Arc::new(DashMap::new());
    let neighbors = Arc::new(Vec::new());
    let backtrace = Arc::new(DashSet::new());
    let listener = tcp::listen((host, args.port), Bincode::default).await?;
    listener
        // Ignore accept errors.
        .filter_map(|r| future::ready(r.ok()))
        // Establish serve channel
        .map(BaseChannel::with_defaults)
        .map(|channel| {
            let server =
                IndexerServer::new(channel.transport().peer_addr().unwrap(), &index, &dl_ports, &neighbors, &backtrace);
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

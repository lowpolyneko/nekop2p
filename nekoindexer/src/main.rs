//! Simple binary wrapping the reference implementation of [IndexerServer] in a
//! [tarpc::serde_transport::tcp::connect].
//!
//! Utilizes two [DashMap]s as the underlying data structure for the [IndexerServer::index] and
//! [IndexerServer::dl_ports].
use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use clap::Parser;
use dashmap::{DashMap, DashSet};
use futures::{future, prelude::*};
use serde::Deserialize;
use tarpc::{
    serde_transport::tcp,
    server::{BaseChannel, Channel},
    tokio_serde::formats::Bincode,
};
use tokio::fs;

use nekop2p::{Indexer, IndexerServer};

#[derive(Deserialize)]
struct Config {
    /// Host to run on
    bind: SocketAddr,

    /// Neighbors of [IndexerServer]
    neighbors: Option<Vec<SocketAddr>>,
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    config: String,
}

/// Starts an [IndexerServer] on [Args::host] with [Args::port]
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let config: Config = toml::from_str(
        &fs::read_to_string(args.config)
            .await
            .expect("missing config file"),
    )
    .expect("failed to parse config file");

    println!("Starting indexer on {0}", config.bind);

    let index = Arc::new(DashMap::new());
    let dl_ports = Arc::new(DashMap::new());
    let neighbors = Arc::new(config.neighbors.unwrap_or_default());
    let backtrace = Arc::new(DashSet::new());
    let listener = tcp::listen(config.bind, Bincode::default).await?;
    listener
        // Ignore accept errors.
        .filter_map(|r| future::ready(r.ok()))
        // Establish serve channel
        .map(BaseChannel::with_defaults)
        .map(|channel| {
            let server = IndexerServer::new(
                channel.transport().peer_addr().unwrap(),
                &index,
                &dl_ports,
                &neighbors,
                &backtrace,
            );
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

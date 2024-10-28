use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use dashmap::DashMap;
use futures::{future, prelude::*};
use serde::Deserialize;
use tarpc::{
    serde_transport::tcp,
    server::{BaseChannel, Channel},
    tokio_serde::formats::Bincode,
};
use tokio::fs;

use nekop2p::{Indexer, IndexerServer, SuperPeer, SuperPeerServer};

#[derive(Deserialize)]
struct Config {
    /// Host
    host: Option<String>,

    /// [IndexerServer] Port
    indexer_port: u16,

    /// [SuperPeerServer] Port
    superpeer_port: u16,
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Config file
    config: String,
}

/// Starts an [IndexerServer] on [Config::host] with [Config::indexer_port] and [SuperPeerServer]
/// on [Config::superpeer_port]
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let config: Config = toml::from_str(
        &fs::read_to_string(args.config)
            .await
            .expect("missing config file"),
    )
    .expect("failed to parse config file");
    let host = config.host.unwrap_or("localhost".to_owned());

    println!("Starting indexer on {0}:{1}", host, config.indexer_port);
    println!("Starting superpeer on {0}:{1}", host, config.superpeer_port);

    let index = Arc::new(DashMap::new());
    let dl_ports = Arc::new(DashMap::new());
    let indexer_listener =
        tcp::listen((host.clone(), config.indexer_port), Bincode::default).await?;
    let indexer = indexer_listener
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
        .for_each(|_| async {});

    let backtrace = Arc::new(DashMap::new());
    let superpeer_listener = tcp::listen((host, config.superpeer_port), Bincode::default).await?;
    let superpeer = superpeer_listener
        // Ignore accept errors.
        .filter_map(|r| future::ready(r.ok()))
        // Establish serve channel
        .map(BaseChannel::with_defaults)
        .map(|channel| {
            let server = SuperPeerServer::new(channel.transport().peer_addr().unwrap(), &backtrace);
            channel
                .execute(server.serve())
                .for_each(|response| async move {
                    tokio::spawn(response);
                })
        })
        // Max 10 channels.
        .buffer_unordered(10)
        .for_each(|_| async {});

    future::join(indexer, superpeer).await;

    Ok(())
}

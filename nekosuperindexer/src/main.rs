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
use tokio::fs;

use nekop2p::{SuperPeer, SuperPeerConfig, SuperPeerServer};

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
    let config: Arc<SuperPeerConfig> = Arc::new(
        toml::from_str(
            &fs::read_to_string(args.config)
                .await
                .expect("missing config file"),
        )
        .expect("failed to parse config file"),
    );
    let host = config.host.clone().unwrap_or("localhost".to_owned());

    println!("Starting superpeer on {0}:{1}", host, config.port);

    let index = Arc::new(DashMap::new());
    let backtrace = Arc::new(DashMap::new());
    let listener = tcp::listen((host, config.port), Bincode::default).await?;
    listener
        // Ignore accept errors.
        .filter_map(|r| future::ready(r.ok()))
        // Establish serve channel
        .map(BaseChannel::with_defaults)
        .map(|channel| {
            let server = SuperPeerServer::new(
                channel.transport().peer_addr().unwrap(),
                &config,
                Some(&index),
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

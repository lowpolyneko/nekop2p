use std::sync::Arc;

use anyhow::Result;
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

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting server on localhost:5000");

    let index = Arc::new(DashMap::new());

    let listener = tcp::listen("localhost:5000", Bincode::default).await?;

        listener
        // Ignore accept errors.
        .filter_map(|r| future::ready(r.ok()))
        // Establish serve channel
        .map(BaseChannel::with_defaults)
        .map(|channel| {
            let server = IndexerServer::new(channel.transport().peer_addr().unwrap(), &index);
            channel.execute(server.serve()).for_each(|response| async move {
                tokio::spawn(response);
            })
        })
        // Max 10 channels.
        .buffer_unordered(10)
        .for_each(|_| async {})
        .await;

    Ok(())
}

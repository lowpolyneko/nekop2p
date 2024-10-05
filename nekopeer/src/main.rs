use anyhow::Result;
use futures::prelude::*;
use tarpc::{
    client, context,
    serde_transport::tcp,
    server::{BaseChannel, Channel},
    tokio_serde::formats::Bincode,
};
use tokio::signal;

use nekop2p::{IndexerClient, Peer};

mod peer;
use crate::peer::PeerServer;

#[tokio::main]
async fn main() -> Result<()> {
    let transport = tcp::connect("localhost:5000", Bincode::default);
    let listener = tcp::listen("localhost:5001", Bincode::default).await?;

    tokio::spawn(
        listener
            // Ignore accept errors.
            .filter_map(|r| future::ready(r.ok()))
            // Establish serve channel
            .map(BaseChannel::with_defaults)
            .map(|channel| {
                channel
                    .execute(PeerServer.serve())
                    .for_each(|response| async move {
                        tokio::spawn(response);
                    })
            })
            // Max 10 channels.
            .buffer_unordered(10)
            .for_each(|_| async {}),
    );

    let client = IndexerClient::new(client::Config::default(), transport.await?).spawn();
    client
        .register(context::current(), "test".to_string())
        .await?;
    client
        .register(context::current(), "test2".to_string())
        .await?;
    client
        .register(context::current(), "test3".to_string())
        .await?;

    // wait for SIGINT
    signal::ctrl_c().await?;

    client
        .deregister(context::current(), "test3".to_string())
        .await?;
    client
        .deregister(context::current(), "test2".to_string())
        .await?;
    client
        .deregister(context::current(), "test".to_string())
        .await?;

    // wait for SIGINT
    signal::ctrl_c().await?;

    Ok(())
}

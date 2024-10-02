use anyhow::Result;
use futures::prelude::*;
use tarpc::{
    serde_transport::tcp,
    tokio_serde::formats::Bincode,
    server::{BaseChannel, Channel},
};

use nekop2p::Indexer;

mod server;
use crate::server::IndexerServer;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting server on localhost:5000");

    let mut listener = tcp::listen("localhost:5000", Bincode::default).await?;

    tokio::spawn(async move {
        let transport = listener.next().await.unwrap().unwrap();
        let server = IndexerServer::new();
        BaseChannel::with_defaults(transport)
            .execute(server.serve())
            .for_each(|response| async move {
                tokio::spawn(response);
            }).await;
    }).await?;

    Ok(())
}

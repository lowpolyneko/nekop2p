use anyhow::Result;
use tarpc::{
    client, context,
    serde_transport::tcp,
    tokio_serde::formats::Bincode,
};

use nekop2p::IndexerClient;

#[tokio::main]
async fn main() -> Result<()> {
    let transport = tcp::connect("localhost:5000", Bincode::default);

    let client = IndexerClient::new(client::Config::default(), transport.await?).spawn();
    client.register(context::current(), "test".to_string()).await?;

    Ok(())
}

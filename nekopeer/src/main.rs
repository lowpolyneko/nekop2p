use std::io::{stdin, stdout, Write};

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

async fn prompt_register(client: &IndexerClient) -> Result<()> {
    let mut filename = String::new();
    stdin().read_line(&mut filename)?;

    client.register(context::current(), filename.trim_end().to_owned()).await?;
    Ok(())
}

async fn prompt_search(client: &IndexerClient) -> Result<()> {
    let mut filename = String::new();
    stdin().read_line(&mut filename)?;

    let results = client.search(context::current(), filename.trim_end().to_owned()).await?;
    results.iter().for_each(|r| println!("{}", r));
    Ok(())
}

async fn prompt_deregister(client: &IndexerClient) -> Result<()> {
    let mut filename = String::new();
    stdin().read_line(&mut filename)?;

    client.deregister(context::current(), filename.trim_end().to_owned()).await?;
    Ok(())
}

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
    client.set_port(context::current(), 5001).await?;

    loop {
        // wait for SIGINT
        signal::ctrl_c().await?;

        // what are we doing?
        print!("Enter Command >> ");
        stdout().flush().unwrap();

        let mut input = String::new();
        stdin().read_line(&mut input)?;

        match input.as_str().trim_end() {
            "register" => prompt_register(&client).await?,
            "search" => prompt_search(&client).await?,
            "deregister" => prompt_deregister(&client).await?,
            "exit" => break,
            _ => {
                println!("unknown command");
                continue;
            }
        }
    }

    Ok(())
}

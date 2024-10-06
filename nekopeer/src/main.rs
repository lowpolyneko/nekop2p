use std::io::{stdin, stdout, Write};

use anyhow::Result;
use clap::Parser;
use futures::prelude::*;
use tarpc::{
    client, context,
    serde_transport::tcp,
    server::{BaseChannel, Channel},
    tokio_serde::formats::Bincode,
};
use tokio::{fs, signal};

use nekop2p::{IndexerClient, Peer, PeerClient};

mod peer;
use crate::peer::PeerServer;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    // indexer
    indexer: String,

    // incoming host
    #[arg(long)]
    dl_host: Option<String>,

    // incoming port
    #[arg(long, default_value_t = 5001)]
    dl_port: u16,
}

fn input(prompt: &str) -> Option<String> {
    // what are we doing?
    print!("{prompt} >> ");
    stdout().flush().unwrap();

    let mut input = String::new();
    match stdin().read_line(&mut input) {
        Ok(_) => Some(input),
        Err(_) => None,
    }
}

async fn prompt_register(client: &IndexerClient) -> Result<()> {
    let filename = input("Enter filename").unwrap();

    client
        .register(context::current(), filename.trim_end().to_owned())
        .await?;
    Ok(())
}

async fn prompt_download(client: &IndexerClient) -> Result<()> {
    let filename = input("Enter filename").unwrap();

    let results = client
        .search(context::current(), filename.trim_end().to_owned())
        .await?;

    // try to download file
    let transport = tcp::connect(results.first().unwrap(), Bincode::default);
    let peer = PeerClient::new(client::Config::default(), transport.await?).spawn();
    let contents = peer
        .download_file(context::current(), filename.trim_end().to_owned())
        .await?;

    fs::write(filename.trim_end(), contents).await?;

    Ok(())
}

async fn prompt_search(client: &IndexerClient) -> Result<()> {
    let filename = input("Enter filename").unwrap();

    let results = client
        .search(context::current(), filename.trim_end().to_owned())
        .await?;
    results.iter().for_each(|r| println!("{}", r));
    Ok(())
}

async fn prompt_deregister(client: &IndexerClient) -> Result<()> {
    let filename = input("Enter filename").unwrap();

    client
        .deregister(context::current(), filename.trim_end().to_owned())
        .await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let dl_host = args.dl_host.unwrap_or("0.0.0.0".to_owned());

    println!("Welcome to nekop2p! (peer client)");
    println!("Press Ctrl-C to enter commands...");
    println!("Connecting to indexer on {0}", args.indexer);
    println!("Accepting inbound connections on {0}:{1}", dl_host, args.dl_port);

    let transport = tcp::connect(args.indexer, Bincode::default);
    let listener = tcp::listen((dl_host, args.dl_port), Bincode::default).await?;

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
    client.set_port(context::current(), args.dl_port).await?;

    loop {
        // wait for SIGINT
        signal::ctrl_c().await?;

        // what are we doing?
        let input = input("Enter Command ('?' for help)").unwrap();

        match input.as_str().trim_end() {
            "register" => prompt_register(&client).await?,
            "download" => prompt_download(&client).await?,
            "search" => prompt_search(&client).await?,
            "deregister" => prompt_deregister(&client).await?,
            "?" => println!("register, download, search, deregister"),
            "exit" => break,
            _ => println!("unknown command"),
        }
    }

    Ok(())
}

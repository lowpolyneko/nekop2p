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

fn print_help() {
    println!("Available CLI commands:");
    println!("register\tRegister file to index");
    println!("download\tDownload file from peer on index");
    println!("search\t\tQuery peers on index");
    println!("deregister\tDeregister file on index");
    println!("?\t\tPrint this help screen");
    println!("exit\t\tQuit");
}

async fn prompt_register(client: &IndexerClient) {
    let filename = input("Enter filename").unwrap();

    match client
        .register(context::current(), filename.trim_end().to_owned())
        .await
    {
        Ok(_) => println!("Registered {0} on index", filename.trim_end()),
        Err(_) => println!("Failed to register {0}", filename.trim_end()),
    }
}

async fn prompt_download(client: &IndexerClient) {
    let filename = input("Enter filename").unwrap();

    let results = match client
        .search(context::current(), filename.trim_end().to_owned())
        .await
    {
        Ok(x) => {
            println!("Querying peers for {0}", filename.trim_end());
            x
        }
        Err(_) => {
            println!("Failed to retrieve peers for {0}", filename.trim_end());
            return;
        }
    };

    // try to download file
    let peer = results.first().unwrap();
    let transport = match tcp::connect(peer, Bincode::default).await {
        Ok(x) => {
            println!("Connecting to peer {0}", peer);
            x
        }
        Err(_) => {
            println!("Failed to connect to peer {0}", peer);
            return;
        }
    };

    let peer = PeerClient::new(client::Config::default(), transport).spawn();
    let contents = match peer
        .download_file(context::current(), filename.trim_end().to_owned())
        .await
    {
        Ok(Some(x)) => {
            println!("Downloading {0}...", filename.trim_end());
            x
        }
        Ok(None) | Err(_) => {
            println!("Failed to download {0}", filename.trim_end());
            return;
        }
    };

    match fs::write(filename.trim_end(), contents).await {
        Ok(_) => println!("Writing contents to {0}...", filename.trim_end()),
        Err(_) => println!("Failed to write to {0}", filename.trim_end()),
    }
}

async fn prompt_search(client: &IndexerClient) {
    let filename = input("Enter filename").unwrap();

    let results = match client
        .search(context::current(), filename.trim_end().to_owned())
        .await
    {
        Ok(x) => {
            println!("Querying peers for {0}", filename.trim_end());
            x
        }
        Err(_) => {
            println!("Failed to retrieve peers for {0}", filename.trim_end());
            return;
        }
    };

    // print out results
    results.iter().for_each(|r| println!("{}", r));
}

async fn prompt_deregister(client: &IndexerClient) {
    let filename = input("Enter filename").unwrap();

    match client
        .deregister(context::current(), filename.trim_end().to_owned())
        .await
    {
        Ok(_) => println!("Deregistered {0} on index", filename.trim_end()),
        Err(_) => println!("Failed to deregister {0}", filename.trim_end()),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let dl_host = args.dl_host.unwrap_or("localhost".to_owned());

    println!("Welcome to nekop2p! (peer client)");
    println!("Press Ctrl-C to enter commands...");
    println!("Connecting to indexer on {0}", args.indexer);
    println!(
        "Accepting inbound connections on {0}:{1}",
        dl_host, args.dl_port
    );

    let transport = tcp::connect(args.indexer, Bincode::default);
    let mut listener = tcp::listen((dl_host, args.dl_port), Bincode::default).await?;
    listener.config_mut().max_frame_length(usize::MAX); // allow large frames

    let port = listener.local_addr().port(); // get port (in-case dl_port = 0)

    tokio::spawn(
        listener
            // Ignore accept errors.
            .filter_map(|r| future::ready(r.ok()))
            // Establish serve channel
            .map(BaseChannel::with_defaults)
            .map(|channel| {
                let server = PeerServer::new(channel.transport().peer_addr().unwrap());
                channel
                    .execute(server.serve())
                    .for_each(|response| async move {
                        tokio::spawn(response);
                    })
            })
            // Max 10 channels.
            .buffer_unordered(10)
            .for_each(|_| async {}),
    );

    let client = IndexerClient::new(client::Config::default(), transport.await?).spawn();
    client.set_port(context::current(), port).await?;

    loop {
        // wait for SIGINT
        signal::ctrl_c().await?;

        // what are we doing?
        let input = input("\nEnter Command ('?' for help)").unwrap();

        match input.as_str().trim_end() {
            "register" => prompt_register(&client).await,
            "download" => prompt_download(&client).await,
            "search" => prompt_search(&client).await,
            "deregister" => prompt_deregister(&client).await,
            "?" => print_help(),
            "exit" => break,
            _ => println!("Unknown command"),
        }
    }

    // ensure the client registrations are cleared
    client.disconnect_peer(context::current()).await?;

    Ok(())
}

//! Simple binary wrapping the reference implementation of [PeerServer] in a
//! [tarpc::serde_transport::tcp::connect]
//!
//! Connects to [nekop2p::Indexer]s and [Peer]s using [IndexerClient] and [PeerClient]
//! respectively.
use std::io::{stdin, stdout, Write};

use anyhow::Result;
use clap::Parser;
use futures::prelude::*;
use rand::seq::SliceRandom;
use serde::Deserialize;
use tarpc::{
    client, context,
    serde_transport::tcp,
    server::{BaseChannel, Channel},
    tokio_serde::formats::Bincode,
};
use tokio::{fs, signal};
use uuid::Uuid;

use nekop2p::{IndexerClient, Peer, PeerClient, PeerServer};

#[derive(Deserialize)]
struct Config {
    /// indexer to bind to
    indexer: String,

    /// incoming peer connection [std::net::SocketAddr] to bind to
    dl_host: String,

    /// incoming peer connection port to bind to
    dl_port: u16,
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    config: String,
}

/// Given a `prompt` read a line from [stdout] and return it if it exists
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

/// Prints all available [crate] commands
fn print_help() {
    println!("Available CLI commands:");
    println!("register\tRegister file to index");
    println!("download\tDownload file from peer on index");
    println!("search\t\tQuery peers on index with file");
    println!("deregister\tDeregister file on index");
    println!("query\tQueries entire network for file");
    println!("?\t\tPrint this help screen");
    println!("exit\t\tQuit");
}

/// Given an [IndexerClient] register a filename that is prompted for
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

/// Given an [IndexerClient] download a file that is prompted for from a random peer and register
/// it with the [nekop2p::Indexer]
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
    let peer = match results.choose(&mut rand::thread_rng()) {
        Some(x) => {
            println!("Selected peer {x}");
            x
        }
        None => {
            println!("No peers to download {0} from", filename.trim_end());
            return;
        }
    };

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

    match client
        .register(context::current(), filename.trim_end().to_owned())
        .await
    {
        Ok(_) => println!("Registered {0} on index", filename.trim_end()),
        Err(_) => println!("Failed to register {0}", filename.trim_end()),
    }
}

/// Given an [IndexerClient] queries all peers for a filename that is prompted for
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

/// Given an [IndexerClient] queries the network for a filename that is prompted for
async fn prompt_query(client: &IndexerClient) {
    let filename = input("Enter filename").unwrap();

    // TODO don't hardcode ttl
    let results = match client
        .query(context::current(), Uuid::new_v4(), filename.trim_end().to_owned(), 1)
        .await
    {
        Ok(x) => {
            println!("Querying network for {0}", filename.trim_end());
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

/// Given an [IndexerClient] deregisters a filename that is prompted for
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

/// Starts a [PeerServer] on [Args::dl_host] with [Args::dl_port] and connects to an
/// [nekop2p::IndexerServer] on [Args::indexer]. Afterwards, the client will enter a REPL with
/// [signal::ctrl_c] indicating when commands should be read.
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let config: Config =
        toml::from_str(
            &fs::read_to_string(args.config)
                .await
                .expect("missing config file"),
        )
        .expect("failed to parse config file");

    println!("Welcome to nekop2p! (peer client)");
    println!("Press Ctrl-C to enter commands...");
    println!("Connecting to indexer on {0}", config.indexer);
    println!(
        "Accepting inbound connections on {0}:{1}",
        config.dl_host, config.dl_port
    );

    let transport = tcp::connect(config.indexer, Bincode::default);
    let mut listener = tcp::listen((config.dl_host, config.dl_port), Bincode::default).await?;
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
            "query" => prompt_query(&client).await,
            "?" => print_help(),
            "exit" => break,
            _ => println!("Unknown command"),
        }
    }

    // ensure the client registrations are cleared
    client.disconnect_peer(context::current()).await?;

    Ok(())
}

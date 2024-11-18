//! Simple binary wrapping the reference implementation of [PeerServer] in a
//! [tarpc::serde_transport::tcp::connect]
//!
//! Connects to [nekop2p::Indexer]s and [Peer]s using [IndexerClient] and [PeerClient]
//! respectively.
use std::{
    io::{stdin, stdout, Write},
    net::SocketAddr,
};

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

use nekop2p::{IndexerClient, Metadata, Peer, PeerClient, PeerServer};

#[derive(Deserialize)]
struct Config {
    /// indexer to bind to
    indexer: SocketAddr,

    /// incoming peer connection [std::net::SocketAddr] to bind to
    dl_bind: SocketAddr,

    /// TTL of queries (default 1)
    ttl: Option<u8>,

    /// TTR of downloads (default 255)
    ttr: Option<u8>,
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

async fn read_metadata(filename: &str) -> Result<Metadata> {
    // get origin server and version from metadata
    let metadata_text = fs::read_to_string(filename.to_owned() + ".meta").await?;
    let metadata: Metadata = toml::from_str(metadata_text.as_str())?;
    Ok(metadata)
}

async fn write_metadata(filename: &str, metadata: &Metadata) {
    // create metadata file
    let metadata_text = match toml::to_string_pretty(&metadata) {
        Ok(x) => {
            println!("Parsing metadata for {0}...", filename.trim_end());
            x
        }
        Err(_) => {
            println!("Failed to parse metadata for {0}", filename.trim_end());
            return;
        }
    };
    match fs::write(filename.trim_end().to_owned() + ".meta", metadata_text).await {
        Ok(_) => println!("Writing metadata for {0}...", filename.trim_end()),
        Err(_) => {
            println!("Failed to write metadata for {0}", filename.trim_end());
            return;
        }
    };
}

/// Given an [IndexerClient] register a filename that is prompted for
async fn prompt_register(client: &IndexerClient, origin_server: SocketAddr, ttr: u8) {
    let filename = input("Enter filename").unwrap();

    // write/get metadata first
    let metadata = match read_metadata(filename.trim_end()).await {
        Ok(x) => x,
        Err(_) => {
            // not found, make new metadata file instead
            let metadata = Metadata {
                origin_server, // this is the origin server!
                version: 0,    // initial version is zero
                ttr,           // we set the ttr
            };
            write_metadata(filename.trim_end(), &metadata).await;
            metadata
        }
    };

    // (try to) invalidate old versions
    match client
        .invalidate(
            context::current(),
            Uuid::new_v4(),
            metadata.origin_server,
            filename.trim_end().to_owned(),
        )
        .await
    {
        Ok(_) => println!("Send update {0}", filename.trim_end()),
        Err(_) => println!("Failed to register {0}", filename.trim_end()),
    }

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
async fn prompt_download(client: &IndexerClient, ttl: u8) {
    let filename = input("Enter filename").unwrap();

    let results = match client
        .query(
            context::current(),
            Uuid::new_v4(),
            filename.trim_end().to_owned(),
            ttl,
        )
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
    let (contents, metadata) = match peer
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
        Err(_) => {
            println!("Failed to write to {0}", filename.trim_end());
            return;
        }
    }

    // create metadata file
    let metadata_text = match toml::to_string_pretty(&metadata) {
        Ok(x) => {
            println!("Parsing metadata for {0}...", filename.trim_end());
            x
        }
        Err(_) => {
            println!("Failed to parse metadata for {0}", filename.trim_end());
            return;
        }
    };
    match fs::write(filename.trim_end().to_owned() + ".meta", metadata_text).await {
        Ok(_) => println!("Writing metadata for {0}...", filename.trim_end()),
        Err(_) => {
            println!("Failed to write metadata for {0}", filename.trim_end());
            return;
        }
    };

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
async fn prompt_query(client: &IndexerClient, ttl: u8) {
    let filename = input("Enter filename").unwrap();

    let results = match client
        .query(
            context::current(),
            Uuid::new_v4(),
            filename.trim_end().to_owned(),
            ttl,
        )
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
    let config: Config = toml::from_str(
        &fs::read_to_string(args.config)
            .await
            .expect("missing config file"),
    )
    .expect("failed to parse config file");

    println!("Welcome to nekop2p! (peer client)");
    println!("Press Ctrl-C to enter commands...");
    println!("Connecting to indexer on {0}", config.indexer);
    println!("Accepting inbound connections on {0}", config.dl_bind);

    let transport = tcp::connect(config.indexer, Bincode::default);
    let mut listener = tcp::listen(config.dl_bind, Bincode::default).await?;
    listener.config_mut().max_frame_length(usize::MAX); // allow large frames

    let ttl = config.ttl.unwrap_or(1);
    let ttr = config.ttr.unwrap_or(255);
    let origin_server = listener.local_addr();
    let port = origin_server.port(); // get port (in-case dl_port = 0)

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
            "register" => prompt_register(&client, origin_server, ttr).await,
            "download" => prompt_download(&client, ttl).await,
            "search" => prompt_search(&client).await,
            "deregister" => prompt_deregister(&client).await,
            "query" => prompt_query(&client, ttl).await,
            "?" => print_help(),
            "exit" => break,
            _ => println!("Unknown command"),
        }
    }

    // ensure the client registrations are cleared
    client.disconnect_peer(context::current()).await?;

    Ok(())
}

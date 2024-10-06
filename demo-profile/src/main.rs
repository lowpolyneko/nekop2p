use std::time::Instant;
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use anyhow::Result;
use clap::Parser;
use dashmap::DashMap;
use futures::{future, prelude::*};
use tarpc::{
    client, context,
    serde_transport::tcp,
    server::{BaseChannel, Channel},
    tokio_serde::formats::Bincode,
};

use nekop2p::{server::IndexerServer, Indexer, IndexerClient};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    indexer: Option<String>,

    #[arg(short, long, default_value_t = 1)]
    concurrent: u32,

    #[arg(short, long, default_value_t = 500)]
    num_requests: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let host = args.indexer.unwrap_or("localhost:5000".to_owned());

    println!("Welcome to the nekop2p profiler!");
    println!("Starting indexer on {0}", host);

    // Start indexer here
    let index = Arc::new(DashMap::new());
    let dl_ports = Arc::new(DashMap::new());
    let listener = tcp::listen(host.clone(), Bincode::default).await?;
    tokio::spawn(
        listener
            // Ignore accept errors.
            .filter_map(|r| future::ready(r.ok()))
            // Establish serve channel
            .map(BaseChannel::with_defaults)
            .map(move |channel| {
                let server =
                    IndexerServer::new(channel.transport().peer_addr().unwrap(), &index, &dl_ports);
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

    // Begin profiling requests
    // Spawn clients
    let mut clients = Vec::new();
    for _ in 0..args.concurrent {
        let transport = tcp::connect(host.clone(), Bincode::default);
        let client = IndexerClient::new(client::Config::default(), transport.await?).spawn();
        clients.push(client);
    }

    // Register binary files on the first peer
    for i in 1..11 {
        clients
            .first()
            .unwrap()
            .register(context::current(), format!("{i}k.bin"))
            .await?;
    }

    // For each round, run a request on each client
    let mutex = Arc::new(RwLock::new(Vec::new()));
    for i in 0..args.num_requests {
        future::join_all(clients.iter().map(|c| async {
            let d = Arc::clone(&mutex);
            let now = Instant::now();
            c.search(context::current(), "1k.bin".to_owned())
                .await
                .expect("failed a query while profiling");
            let elapsed = now.elapsed();
            {
                d.write().unwrap().push(elapsed);
            }
            println!("Run {}: {:0.2?}", i, elapsed);
        }))
        .await;
    }

    let durations = mutex.read().unwrap();
    let total = durations
        .iter()
        .sum::<Duration>()
        .div_f64(durations.len() as f64);
    println!("Average time: {:0.2?}", total);
    Ok(())
}

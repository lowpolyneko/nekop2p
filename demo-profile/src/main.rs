//! Simple profiler that runs tests the `search` query in the nekop2p RPC.
//!
//! The compiled binary uses `-c` for the number of concurrent clients and `-n` for the number of
//! requests to run and simulates `c*n` search queries on a dummy [IndexerServer].
//!
//! Additionally, plots can be generated using the [plotly] crate.
use std::iter::repeat;
use std::time::Instant;
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use anyhow::Result;
use clap::Parser;
use dashmap::{DashMap, DashSet};
use futures::{future, prelude::*};
use plotly::common::Mode;
use plotly::Histogram;
use plotly::{layout::Axis, Layout, Plot, Scatter};
use tarpc::{
    client, context,
    serde_transport::tcp,
    server::{BaseChannel, Channel},
    tokio_serde::formats::Bincode,
};

use nekop2p::{Indexer, IndexerClient, IndexerServer};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// IP and port to bind the [IndexerServer] to
    indexer: Option<String>,

    /// Whether or not to plot
    #[arg(short, long, action)]
    plot: bool,

    /// Number of concurrent clients
    #[arg(short, long, default_value_t = 1)]
    concurrent: usize,

    /// Number of request rounds to run
    #[arg(short, long, default_value_t = 500)]
    num_requests: usize,
}

/// Sets-up an [IndexerServer], with [Args::concurrent] clients and runs [Args::num_requests]
/// rounds, optionally plotting if [Args::plot] is set
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let host = args.indexer.unwrap_or("localhost:5000".to_owned());

    println!("Welcome to the nekop2p profiler!");
    println!("Starting indexer on {0}", host);

    // Start indexer here
    let index = Arc::new(DashMap::new());
    let dl_ports = Arc::new(DashMap::new());
    let neighbors = Arc::new(Vec::new());
    let backtrace = Arc::new(DashSet::new());
    let listener = tcp::listen(host.clone(), Bincode::default).await?;
    tokio::spawn(
        listener
            // Ignore accept errors.
            .filter_map(|r| future::ready(r.ok()))
            // Establish serve channel
            .map(BaseChannel::with_defaults)
            .map(move |channel| {
                let server =
                    IndexerServer::new(channel.transport().peer_addr().unwrap(), &index, &dl_ports, &neighbors, &backtrace);
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
    println!("Spawning {0} clients", args.concurrent);
    let mut clients = Vec::new();
    for _ in 0..args.concurrent {
        let transport = tcp::connect(host.clone(), Bincode::default);
        let client = IndexerClient::new(client::Config::default(), transport.await?).spawn();
        clients.push(client);
    }

    // Register binary files on the first peer
    for i in 1..=10 {
        println!("Registering {i}k.bin on first peer");
        clients
            .first()
            .unwrap()
            .register(context::current(), format!("{i}k.bin"))
            .await?;
    }

    // For each round, run a request on each client
    println!("Starting runs!");
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

    // plot?
    if args.plot {
        let mut plot = Plot::new();
        let x_axis = (1..=args.num_requests)
            .flat_map(|x| repeat(x).take(args.concurrent))
            .collect();
        let y_axis: Vec<_> = durations.iter().map(|d| d.as_micros()).collect();
        let trace = Scatter::new(x_axis, y_axis.clone())
            .name("Raw Data")
            .mode(Mode::Markers);
        plot.add_trace(trace);

        let x_avg_axis = (1..=args.num_requests).collect();
        let y_avg_axis = durations
            .chunks(args.concurrent)
            .map(|i| {
                i.iter()
                    .sum::<Duration>()
                    .div_f64(args.concurrent as f64)
                    .as_micros()
            })
            .collect();
        let trace_avg = Scatter::new(x_avg_axis, y_avg_axis)
            .name("Average per Request")
            .mode(Mode::Lines);
        plot.add_trace(trace_avg);

        let layout = Layout::new()
            .title("`search` Response Time")
            .x_axis(Axis::new().title("Request Iteration"))
            .y_axis(Axis::new().title("Response Time (microseconds)"));
        plot.set_layout(layout);

        plot.show();

        let mut histogram = Plot::new();
        let hist_trace = Histogram::new(y_axis);
        histogram.add_trace(hist_trace);

        let hist_layout = Layout::new()
            .title("Distribution of `search` Response Time")
            .x_axis(
                Axis::new()
                    .title("Response Time (microseconds)")
                    .range(vec![0, 250]),
            )
            .y_axis(Axis::new().title("Count"));
        histogram.set_layout(hist_layout);

        histogram.show();
    }

    Ok(())
}

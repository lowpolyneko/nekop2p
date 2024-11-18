# Design

## New: Consistency
In `nekop2p` v0.3.0, there are new consistency checks added. These checks are
done through a push and pull method. For the *push* approach, a new `invalidate`
RPC scheme is added.

```rs
    /// Invalidates a `filename` on endpoint, discarding if request is from the
    /// origin
    async fn invalidate(msg_id: Uuid, origin_server: SocketAddr, filename: String);
```

This `invalidate` is now sent on every `nekopeer` client registration, meaning
that every `register` command will propagate an `invalidate` across the entire
network, causing all previously cached indexer entries to be dropped and for
clients to auto delete their downloaded version. Since only the `origin_server`
will ever send invalidations, sending a version number is unnecessary as a
re`register` will always prune indexer entries, meaning other clients are forced
to re-download regardless if the version number remains the same. If clients
wish, they can subsequently re`download` the file for the updated file and
corresponding metadata.

Invalidation is not indefinitely propagated over the network as there is a
`msg_id` UUID passed which is checked for uniqueness before the RPC is acted
upon.

```rs
        // if msg_id has already been seen, then we ignore the query
        if self.backtrace.read().await.contains_key(&msg_id) {
            println!("Message {msg_id} already handled!");
            return;
        }

        // insert into set of seen msg_ids
        self.backtrace.write().await.insert(msg_id);
```

Files have metadata introduced in the form of `.toml` serialized `.meta` files
stored on the filesystem next to the actual file. Implementation of this file is
done through `serde` serialization.

```rs
/// [Peer] downloaded file metadata
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Metadata {
    /// Server the file originated from (not necessarily downloaded)
    pub origin_server: SocketAddr,

    /// Version number of the file
    pub version: u8,

    /// TTR of the file, or when to check for validity
    pub ttr: u8,
}
```

This metadata is needed for the *pull* approach, which requires all three fields
to poll for updates to the original file from the `origin_server`. Clients send
a TTR of `255` by default with newly download files and will increment `version`
every re`register`, meaning that polled files will be discarded by a
`poll_file_validity` `tokio` task.

```rs
/// Check file validity after ttr
async fn poll_file_validity(filename: String, metadata: Metadata);
```

This routine sleeps for `ttr` seconds before re-downloading the file's metadata
from the `origin_server` with a new `get_metadata` RPC.

```rs
    /// Poll file metadata
    async fn get_metadata(filename: String) -> Option<Metadata>;
```

If this file metadata is different from the one moved into `poll_file_validity`,
the original file and its corresponding `.meta` file is deleted off the client,
requiring the user to re`download` the file for it to be re`register`ed to its
indexer. In this case, the polling is done eagerly as soon as the file's TTR is
reached by the client.


## New: Superpeering
With `nekop2p` v0.2.0, indexers gain the ability to act as superpeers in a
Gnutella-esque fully distributed all-to-all network model. The crux of this
implementation is a new RPC call `query` which in principle acts as an index
search but with propagation properties. 

Snippet of the RPC declaration.
```rs
/// Queries entire network for `filename` with a given ttl
async fn query(msg_id: Uuid, filename: String, ttl: u8) -> Vec<SocketAddr>;
```

Queries are tagged with a `Uuid::v4` to uniquely identify them throughout the
entire network. UUIDs were chosen given their negligible change of collision and
ease of serialization.

Entire definition.
```rs
    async fn query(self, c: Context, msg_id: Uuid, filename: String, ttl: u8) -> Vec<SocketAddr> {
        println!("Querying {filename} for {0} (id: {msg_id})", self.addr);
        // if msg_id has already been seen, then we ignore the query
        if self.backtrace.read().await.contains_key(&msg_id) {
            println!("Message {msg_id} already handled!");
            return Vec::new();
        }

        // insert into set of seen msg_ids
        self.backtrace.write().await.insert(msg_id);

        // get peers from this peer's index
        println!("Searched {filename} for {0}", self.addr);
        let mut peers: Vec<_> = self
            .index
            .entry(filename.clone())
            .or_default()
            .iter()
            .filter_map(|e| match self.dl_ports.get(&e) {
                Some(x) => {
                    let mut n = e.clone();
                    n.set_port(*x);
                    Some(n)
                }
                None => None,
            })
            .collect();

        // propogate query to neighboring peers
        if ttl > 0 {
            for peer in self.neighbors.iter() {
                println!(
                    "Propagating query of {filename} to {0} (id: {msg_id})",
                    peer
                );
                if let Ok(transport) = tcp::connect(peer, Bincode::default).await {
                    let client = IndexerClient::new(client::Config::default(), transport).spawn();
                    peers.append(
                        &mut client
                            .query(c, msg_id, filename.clone(), ttl - 1)
                            .await
                            .unwrap_or_default(),
                    );
                }
            }
        }

        peers
    }
```

UUIDs are used to enforce at most one propagation of a query per
indexer/superpeer. In the current implementation, these UUIDs are saved as
elements in an `HashSetDelay` from `delay_map`, which provides a `HashSet` with
expiring entries given a `ttl` (default $10$ seconds).

Queries are recursively propagated with a `ttl` argument, with zero as its
base-case. When a node encounters a `query` with a non-zero `ttl`, the request
is propagated by connecting to all the neighbors of the node stored in
`self.neighbors: Vec<SocketAddr>` and sending a query with `ttl - 1`. This is
continued until the `ttl` becomes zero, which then back-propagates the result of
the index query back to the original caller, cascading up the call-stack while
coalescing the resulting index hits. Upon an index miss, an empty list is
back-propagated, indicating no match. `tarpc` handles back-propagation of return
values to the caller, eliminating the need for manual tracking or a
`query_hit` return RPC.

A client can then download the file using the `PeerClient::download_file` interface
with one of the returned peer `SocketAddr` as its address.

Both the client and indexer are now configured using
[`.toml`](https://toml.io/en/A). `.toml` enjoys first-class support in Rust with
direct serialization to structs from a `File` using `toml` and `serde`.

Snippet of `Config` file declaration.
```rs
#[derive(Deserialize)]
struct Config {
    /// Host to run on
    bind: SocketAddr,

    /// Neighbors of [IndexerServer]
    neighbors: Option<Vec<SocketAddr>>,

    /// Query Backtrace TTL (default 10 seconds)
    ttl: Option<u64>,
}
```

And snippet to load `Config` file.
```rs
    let config: Config = toml::from_str(
        &fs::read_to_string(args.config)
            .await
            .expect("missing config file"),
    )
    .expect("failed to parse config file");
```

### Limitations
Query propagation in this style carries some drawbacks...

- Indexers/superpeers can be easily DDOS'd by sending a query to one superpeer
  with an absurdly high `ttl`, causing an indefinite propagation (and likely a
  failed request).
- Back-propagation is slightly inefficient as peers will *always* respond to a
  query, even on failure. An ideal system should likely treat *no response* as
  the failure/empty response to save bandwidth.
- With the current back-propagation scheme, a `ttl` > 2 is not viable even with
  a small (<= 10) pool of superpeers when connected **all-to-all**, given the
  exponential growth of query requests propagated as a result.
- Static definition of the network is required as there are no discovery methods
  for peers, meaning a `config.toml` or equivalent must be defined for each
  superpeer to know one another.
- `delay_map` is marginally more inefficient than `DashSet` due to the
  requirement of wrapping around a `RwLock<T>`, in practice this should be a
  real in-memory database for improved concurrency.

## Philosophy
As stated in the `README.md`, `nekop2p` is a simple file sharing implementation
built on `tokio` and `tarpc`. Using these well-known crates and Rust's amazing
concurrency model, it is possible to build a networked P2P application utilizing
fairly little code.

## Structure
This project is decomposed into four crates (akin to Python wheels) using Rust's
workspace model. Listed are the crates in `nekop2p`. Each one has its own
`Cargo.toml` file specifying its corresponding dependencies.
- `demo-profile`
- `nekoindexer`
- `nekop2p`
- `nekopeer`

## Implementation
lib`nekop2p` implements the reference `PeerServer` and `IndexerServer`
implementations, which `nekopeer` and `nekoindexer` utilize. Both `nekopeer` and
`nekoindexer` use Tokio's wrapper `TCPListener` and `TCPStream` to serve
multiple clients and a server simultaneously.

For the backend, `nekoindexer` maps connected peer IPs to files using two
concurrent-aware `HashMap`s. When a RPCs are made, these `HashMap`s are
looked up or modified to return the expected result.

### Tokio
Tokio is an *async/await runtime* which utilizes a shared thread-pool model to
enable concurrency. Tokio is an extremely popular crate, and there exists alot
of community supported tooling. For `nekop2p`, there is heavy utilization of
Tokio modules, notably `tokio::fs` for async file I/O in `nekopeer`, and
concurrent TCP connections are handled using `tokio::spawn` for both `nekoindex`
and `nekopeer`.

Snippet of `nekopeer`'s socket setup code.
```rs
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
```

This, alongside Rust's built-in concurrency primitives like `Arc<T>` and
community crates like `dashmap`, a concurrent aware Hashmap, allows for easy
sharing of data across threads with compile-time guarantees against race
conditions or deadlocks.

### Tarpc
Tarpc is an RPC framework designed for Rust. Tarpc differs from other RPC
frameworks in that the scheme is defined *entirely in Rust*.

Snippet of shared scheme definitions in lib`nekop2p`.
```rs
#[tarpc::service]
pub trait Indexer {
    async fn set_port(dl_port: u16);
    async fn register(filename: String);
    async fn search(filename: String) -> Vec<SocketAddr>;
    async fn deregister(filename: String);
    async fn disconnect_peer();
}

#[tarpc::service]
pub trait Peer {
    async fn download_file(filename: String) -> Option<Vec<u8>>;
}
```

Tarpc uses Rust macros (`#[...]`) to generate Client and Server interfaces which
can then be implemented to provide the corresponding RPC functionality.

Snippet of `PeerServer` implementation in lib`nekop2p`.
```rs
impl Peer for PeerServer {
    async fn download_file(self, _: Context, filename: String) -> Option<Vec<u8>> {
        println!(
            "Handling download request for {0} from {1}",
            filename, self.addr
        );
        fs::read(filename).await.ok()
    }
}
```

Snippet of `IndexerClient` usage in `nekopeer`.
```rs
    let client = IndexerClient::new(client::Config::default(), transport.await?).spawn();
    client.set_port(context::current(), port).await?;
```

## Limitations
There are some limitations that exist in the current implementation, which are
listed below.

- For time-constraint sake, both `nekopeer` and `nekoindexer` are limited to
  `10` connections each. For simple demonstrations this is alright, but in
  practice this number needs to be far higher.
- `nekoindexer` does not prune the index if a `nekopeer` suddenly drops
  connection. In the current implementation, `nekopeer` voluntarily calls an RPC
  to deregister itself. In practice, however, this needs to be done on the
  server in the case of a malicious client.
- `nekoindexer` does no form of file checking, meaning a `nekopeer` can
  `register` an arbitrary file and name it as something else on the index,
  tricking other peers to download a different file than expected. This can be
  solved like in BitTorrent using cryptographic hashes to validate files.
- Lack of chunking in `nekopeer` means that large files will likely fail to
  transfer in practice, a chunking implementation needs to be implemented for
  large file transfers to be reliable.
- The underlying RPC transport lacks security. Maliciously crafted clients can
  likely abuse the RPC calls to hang clients, download files other than what's
  indexed, DOS the indexer with junk registrations, etc.
- Something better than a `DashMap<K, V>` should be used like an in-memory DB
  (i.e. Redis) for real implementations of an Indexer where performance matters.
- The current implementation relies on the TCP connection between the peer and
  indexer being long-living. Real implementations should instead allow for
  multiple connects and disconnects and track clients in-spite of this.

<!-- vim: set tw=80:
-->

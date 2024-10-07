# Design
## Philosophy
As stated in the `README.md`, `nekop2p` is a simple file sharing implementation
built on `tokio` and `tarpc`. Using these well-known crates and Rust's amazing
concurrency model, it is possible to build a networked P2P application utilizing
fairly little code.

## Structure
This project is decomposed into four crates (akin to Python wheels) using Rust's
workspace model. Listed are the crates in `nekop2p`. Each one has its own
`Cargo.toml` file specifying its corrisponding dependencies.
- `demo-profile`
- `nekoindexer`
- `nekop2p`
- `nekopeer`

## Tokio
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

## Tarpc
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

Tarpc uses Rust macros (`$[...]`) to generate Client and Server interfaces which
can then be implemented to provide the corrisponding RPC functionality.

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

<!-- vim: set tw=80:
-->

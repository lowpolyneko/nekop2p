use anyhow::Result;
use futures::prelude::*;
use tarpc::server::{BaseChannel, Channel};
use tarpc::transport::channel::unbounded;
use tokio::spawn;

use nekop2p::Indexer;

mod server;
use crate::server::IndexerServer;

#[tokio::main]
async fn main() -> Result<()> {
    let (_, server_transport) = unbounded();

    let server = BaseChannel::with_defaults(server_transport);
    let _ = spawn(
        server
            .execute(IndexerServer.serve())
            .for_each(|response| async move {
                spawn(response);
            }),
    )
    .await?;

    Ok(())
}

use std::net::SocketAddr;

use serde::{Deserialize, Serialize};
use tarpc::context::Context;
use tokio::fs;

use crate::Peer;

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

/// Reference [Peer] implementation
#[derive(Clone)]
pub struct PeerServer {
    /// Address of remote peer
    addr: SocketAddr,
}

impl PeerServer {
    /// Create a new [PeerServer] with the address of the remote peer
    pub fn new(addr: SocketAddr) -> Self {
        PeerServer { addr }
    }
}

impl Peer for PeerServer {
    async fn download_file(self, _: Context, filename: String) -> Option<Vec<u8>> {
        println!(
            "Handling download request for {0} from {1}",
            filename, self.addr
        );
        match fs::read(filename).await {
            Ok(x) => Some(x),
            Err(_) => None,
        }
    }

    async fn invalidate(
        self,
        _: Context,
        _: uuid::Uuid,
        origin_server: SocketAddr,
        filename: String,
    ) {
        // get origin server and version from metadata
        let metadata_text = match fs::read_to_string(filename.clone() + ".meta").await {
            Ok(x) => x,
            Err(_) => return,
        };
        let metadata: Metadata = match toml::from_str(metadata_text.as_str()) {
            Ok(x) => x,
            Err(_) => return,
        };

        // remove if origin server matches
        if origin_server == metadata.origin_server {
            // got an invalidation message of a file, assume file is bad and delete
            println!(
                "Recieved invalidation message for {0}::{1} from {2}",
                filename, origin_server, self.addr
            );
            let _ = fs::remove_file(filename.clone()).await;
            let _ = fs::remove_file(filename + ".meta").await;
        } else {
            println!(
                "Recieved invalid invalidation message for {0} from {2} with bad origin {1}",
                filename, origin_server, self.addr
            );
        }
    }

    async fn get_metadata(self, _: Context, filename: String) -> Option<Metadata> {
        // get origin server and version from metadata
        let metadata_text = match fs::read_to_string(filename.clone() + ".meta").await {
            Ok(x) => x,
            Err(_) => return None,
        };
        toml::from_str(metadata_text.as_str()).ok()
    }
}

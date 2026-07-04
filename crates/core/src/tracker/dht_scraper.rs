use mainline::{Dht, Id};
use std::net::SocketAddr;
use tokio::sync::mpsc;

pub struct DhtScraper;

impl DhtScraper {
    pub fn start(info_hash: [u8; 20], ip_tx: mpsc::Sender<SocketAddr>) {
        tokio::spawn(async move {
            let dht = match Dht::client() {
                Ok(d) => d,
                Err(_e) => {
                    // println!("[DHT] Failed to start DHT client: {}", e);
                    return;
                }
            };

            let id = Id::from_bytes(info_hash).unwrap();
            let mut stream = dht.as_async().get_peers(id);

            // Need StreamExt to use next()
            use futures_lite::stream::StreamExt;

            // println!("[DHT] Started searching for peers...");
            while let Some(peers_v4) = stream.next().await {
                // println!("[DHT] Found {} peers", peers_v4.len());
                for peer_v4 in peers_v4 {
                    let addr = SocketAddr::V4(peer_v4);
                    if ip_tx.send(addr).await.is_err() {
                        return; // Manager shut down
                    }
                }
            }
            // println!("[DHT] Search stream ended.");
        });
    }
}

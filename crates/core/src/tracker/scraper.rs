use crate::tracker::udp::UdpTrackerClient;
use crate::tracker::{AnnounceRequest, TrackerEvent};
use std::net::SocketAddr;
use tokio::sync::mpsc;

pub struct TrackerScraper;

impl TrackerScraper {
    /// Spawns parallel background tasks to blast UDP Announce requests to all trackers at once.
    /// Valid Peer IPs are streamed back into the `ip_tx` channel immediately as trackers respond.
    pub fn start(
        trackers: Vec<String>,
        info_hash: [u8; 20],
        peer_id: [u8; 20],
        ip_tx: mpsc::Sender<SocketAddr>,
    ) {
        for url in trackers {
            let tx = ip_tx.clone();

            // Spawn an isolated green thread for every single tracker URL.
            // If one tracker takes 10 seconds to timeout, it has zero impact on the others.
            tokio::spawn(async move {
                let mut req = AnnounceRequest {
                    info_hash,
                    peer_id,
                    port: 6881, // Our client's listening port
                    uploaded: 0,
                    downloaded: 0,
                    left: 0, // Unknown for magnet
                    event: TrackerEvent::Started,
                };

                loop {
                    let announce_result = if url.starts_with("udp://") {
                        let host = url.replace("udp://", "").replace("/announce", "");
                        let client = UdpTrackerClient::new(host.clone());
                        client.announce(&req).await
                    } else if url.starts_with("http://") || url.starts_with("https://") {
                        let client = crate::tracker::http::HttpTrackerClient::new(url.clone());
                        client.announce(&req).await
                    } else {
                        return; // Unsupported protocol
                    };

                    // Execute the full Connect + Announce handshake
                    match announce_result {
                        Ok(response) => {
                            // println!("[Tracker: {}] Discovered {} peers!", url, response.peers.len());
                            // As soon as this specific tracker replies, stream the IPs to the Torrent Manager
                            for peer_ip in response.peers {
                                // If the manager shut down, the channel is closed and this safely fails.
                                if tx.send(peer_ip).await.is_err() {
                                    return; // Manager shut down, exit loop
                                }
                            }
                        }
                        Err(_e) => {
                            // println!("[Tracker: {}] Failed: {}", url, _e);
                        }
                    }

                    req.event = TrackerEvent::None; // Change to None after first announce

                    // Aggressive DHT/Tracker behavior: re-announce every 60s
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                }
            });
        }
    }
}

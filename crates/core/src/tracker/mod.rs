pub mod dht_scraper;
use std::net::SocketAddr;

pub mod http;
pub mod scraper;
pub mod udp;

/// Standard BitTorrent events sent to the tracker.
pub enum TrackerEvent {
    None = 0,
    Completed = 1,
    Started = 2,
    Stopped = 3,
}

/// The payload we send to the tracker to ask for peers.
pub struct AnnounceRequest {
    pub info_hash: [u8; 20],
    pub peer_id: [u8; 20],
    pub port: u16, // The port our client is listening on
    pub uploaded: u64,
    pub downloaded: u64,
    pub left: u64,
    pub event: TrackerEvent,
}

/// The standardized response we expect back from ANY tracker (UDP or HTTP).
pub struct AnnounceResponse {
    pub interval: u32,          // How many seconds to wait before announcing again
    pub seeders: u32,           // Peers with 100% of the file
    pub leechers: u32,          // Peers actively downloading
    pub peers: Vec<SocketAddr>, // The gold mine: IPs and Ports to connect to
}

/// A unified interface for interacting with different Tracker protocols (UDP, HTTP).
#[allow(async_fn_in_trait)]
pub trait TrackerClient: Send + Sync {
    async fn announce(
        &self,
        req: &AnnounceRequest,
    ) -> Result<AnnounceResponse, crate::error::TorryError>;
}

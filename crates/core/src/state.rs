use napi_derive::napi;
use std::collections::HashMap;

#[napi(object)]
#[derive(Clone, Default, Debug)]
pub struct PeerStateSnapshot {
    pub ip: String,
    pub unchoked: bool,
    pub blocks_in_flight: u32,
}

/// A lock-free, zero-cost snapshot of the engine's current state.
/// This object is returned to V8 (Node.js/TypeScript) via NAPI.
#[napi(object)]
#[derive(Clone, Default, Debug)]
pub struct TorrentSnapshot {
    pub info_hash: String,
    pub total_downloaded: f64,
    pub total_uploaded: f64,
    pub download_speed: f64, // Bytes per second
    pub upload_speed: f64,
    pub active_peers: u32,
    pub state_string: String, // e.g., "FETCHING_METADATA", "DOWNLOADING"
    pub torrent_name: String,
    pub magnet_uri: String,
    pub progress: f64,
    pub source: String,
    pub total_bytes: f64,
    pub download_speed_limit: f64, // 0.0 means unlimited
    pub upload_speed_limit: f64,   // 0.0 means unlimited
    pub peers: Vec<PeerStateSnapshot>,
    pub piece_map: Vec<u32>,
    pub total_pieces: u32,
}

#[napi(object)]
#[derive(Clone, Default, Debug)]
pub struct EngineSnapshot {
    pub torrents: HashMap<String, TorrentSnapshot>,
}

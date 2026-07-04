//! NAPI-RS binding layer for Torry.
//!
//! This crate is an **ultra-thin bridge** between Node.js and `torry-core`.
//! It contains ONLY `#[napi]` function signatures and type conversions.
//! All business logic lives in `torry-core`.
//!
//! # Rules for this crate
//!
//! 1. **No business logic** — delegate everything to `torry_core`.
//! 2. **Keep functions small** — convert types, call core, return result.
//! 3. **One `#[napi]` fn** per core function that needs JS exposure.
//! 4. **Error conversion** — map `torry_core::TorryError` to `napi::Error`.

use napi_derive::napi;

#[napi(object)]
pub struct PeerStateSnapshot {
    pub ip: String,
    pub unchoked: bool,
    pub blocks_in_flight: u32,
}

#[napi(object)]
pub struct TorrentSnapshot {
    pub info_hash: String,
    pub total_downloaded: f64,
    pub total_uploaded: f64,
    pub download_speed: f64,
    pub upload_speed: f64,
    pub active_peers: u32,
    pub state_string: String,
    pub torrent_name: String,
    pub progress: f64,
    pub peers: Vec<PeerStateSnapshot>,
    pub piece_map: Vec<u32>,
    pub total_pieces: u32,
    pub total_bytes: f64,
    pub source: String,
}

#[napi(object)]
pub struct EngineSnapshot {
    pub torrents: Vec<TorrentSnapshot>,
}

#[napi]
pub struct RuntimeContext {
    pub name: String,
    pub db_path: String,
    pub torrents_dir: String,
    pub downloads_dir: String,
    pub cache_dir: String,

    #[napi(skip)]
    pub inner: torry_core::profile::RuntimeContext,
}

#[napi]
impl RuntimeContext {
    #[napi(constructor)]
    pub fn new(profile_name: String) -> napi::Result<Self> {
        let inner = torry_core::profile::RuntimeContext::new(profile_name)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;

        Ok(Self {
            name: inner.name.clone(),
            db_path: inner.db_path.clone(),
            torrents_dir: inner.torrents_dir.clone(),
            downloads_dir: inner.downloads_dir.clone(),
            cache_dir: inner.cache_dir.clone(),
            inner,
        })
    }

    #[napi]
    pub fn get_snapshot(&self) -> EngineSnapshot {
        let snap = self.inner.get_snapshot();
        let mut torrents = Vec::new();
        for (_, ts) in snap.torrents {
            torrents.push(TorrentSnapshot {
                info_hash: ts.info_hash,
                total_downloaded: ts.total_downloaded,
                total_uploaded: ts.total_uploaded,
                download_speed: ts.download_speed,
                upload_speed: ts.upload_speed,
                active_peers: ts.active_peers,
                state_string: ts.state_string,
                torrent_name: ts.torrent_name,
                progress: ts.progress,
                peers: ts
                    .peers
                    .into_iter()
                    .map(|p| PeerStateSnapshot {
                        ip: p.ip,
                        unchoked: p.unchoked,
                        blocks_in_flight: p.blocks_in_flight,
                    })
                    .collect(),
                piece_map: ts.piece_map,
                total_pieces: ts.total_pieces,
                total_bytes: ts.total_bytes,
                source: ts.source,
            });
        }
        EngineSnapshot { torrents }
    }

    #[napi]
    pub fn add_magnet(&self, uri: String, source: Option<String>) -> napi::Result<()> {
        self.inner
            .add_magnet(uri, source)
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    #[napi]
    pub fn cancel_torrent(&self, info_hash: String) -> napi::Result<()> {
        self.inner
            .cancel_torrent(info_hash)
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }
}

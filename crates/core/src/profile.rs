use crate::download::manager::TorrentManagerActor;
use crate::tracker::scraper::TrackerScraper;
use directories::{ProjectDirs, UserDirs};
use fs2::FileExt;
use napi_derive::napi;
use std::fs::{self, File};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

/// The root dependency container.
/// Every subsystem (DB, Cache, Downloads) must derive its paths from here.
#[napi]
pub struct RuntimeContext {
    pub name: String,
    pub db_path: String,
    pub torrents_dir: String,
    pub downloads_dir: String,
    pub cache_dir: String,

    // We keep the file open to maintain the OS-level lock.
    // It is released automatically when the process dies or context drops.
    #[napi(skip)]
    pub lock: Option<File>,

    #[napi(skip)]
    pub rt: tokio::runtime::Runtime,

    #[napi(skip)]
    pub db_pool: sqlx::SqlitePool,

    #[napi(skip)]
    pub shared_state: Arc<RwLock<crate::state::EngineSnapshot>>,

    #[napi(skip)]
    pub incoming_routers: Arc<RwLock<std::collections::HashMap<[u8; 20], mpsc::Sender<(tokio::net::TcpStream, [u8; 8], [u8; 20])>>>>,

    #[napi(skip)]
    pub download_speed_limit: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

fn decode_hex(s: &str) -> Result<[u8; 20], String> {
    if s.len() != 40 {
        return Err("Info hash must be 40 hex characters".into());
    }
    let mut bytes = [0u8; 20];
    for i in 0..20 {
        let chunk = &s[i * 2..i * 2 + 2];
        bytes[i] = u8::from_str_radix(chunk, 16).map_err(|_| "Invalid hex".to_string())?;
    }
    Ok(bytes)
}

#[napi]
impl RuntimeContext {
    /// Allows the TypeScript TUI to poll the Rust engine state without locking.
    #[napi]
    pub fn get_snapshot(&self) -> crate::state::EngineSnapshot {
        self.shared_state.read().unwrap().clone()
    }

    /// Sets the global download speed limit in bytes per second.
    /// Pass 0 to disable the limit (unlimited).
    #[napi]
    pub fn set_download_speed_limit(&self, bytes_per_second: f64) {
        self.download_speed_limit.store(
            bytes_per_second as u64,
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    /// Gets the current download speed limit in bytes per second.
    /// Returns 0 if unlimited.
    #[napi]
    pub fn get_download_speed_limit(&self) -> f64 {
        self.download_speed_limit
            .load(std::sync::atomic::Ordering::Relaxed) as f64
    }

    /// Parses a magnet link and spawns the TorrentManagerActor triage pipeline
    #[napi]
    pub fn add_magnet(&self, uri: String, source: Option<String>) -> napi::Result<()> {
        let parsed = crate::magnet::parse_magnet_uri(uri.clone())
            .map_err(|e| napi::Error::from_reason(format!("Invalid Magnet URI: {}", e)))?;

        let raw_info_hash = decode_hex(&parsed.info_hash).map_err(napi::Error::from_reason)?;

        if let Ok(st) = self.shared_state.read()
            && let Some(t) = st.torrents.get(&parsed.info_hash)
                && t.state_string != "PAUSED" && !t.state_string.starts_with("ERROR") {
                    return Ok(()); // Already running
                }

        let peer_id: [u8; 20] = rand::random();
        let downloads_dir = self.downloads_dir.clone();
        let shared_state = self.shared_state.clone();
        let download_speed_limit = self.download_speed_limit.clone();
        let info_hash_hex = parsed.info_hash.clone();
        let source_str = source.clone().unwrap_or_else(|| "Unknown".to_string());

        let session = crate::session::TorrentSession {
            info_hash: info_hash_hex.clone(),
            display_name: parsed.display_name.clone(),
            magnet_uri: uri.clone(),
            source: source.clone(),
            state: crate::session::SessionState::PendingMetadata,
            added_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        };
        if let Err(_e) = self
            .rt
            .block_on(crate::db::create_session(&self.db_pool, &session))
        {
            // eprintln!("Failed to save session: {}", e);
        }

        if let Ok(mut st) = self.shared_state.write() {
            st.torrents.insert(
                info_hash_hex.clone(),
                crate::state::TorrentSnapshot {
                    info_hash: info_hash_hex.clone(),
                    torrent_name: parsed
                        .display_name
                        .clone()
                        .unwrap_or_else(|| format!("Magnet: {}", info_hash_hex)),
                    magnet_uri: uri.clone(),
                    state_string: "FETCHING_METADATA".to_string(),
                    source: source_str,
                    total_bytes: 0.0,
                    ..Default::default()
                },
            );
        }

        let (incoming_tx, incoming_rx) = mpsc::channel(100);
        {
            let mut routers = self.incoming_routers.write().unwrap();
            routers.insert(raw_info_hash, incoming_tx);
        }

        let db_pool = self.db_pool.clone();
        
        self.rt.spawn(async move {
            let (ip_tx, ip_rx) = mpsc::channel(4096);
            let (event_tx, event_rx) = mpsc::channel(16384); // Peer event channel

            let (disk_cmd_tx, disk_cmd_rx) = mpsc::channel(8192);
            let (disk_event_tx, disk_event_rx) = mpsc::channel(8192);

            let disk_actor = crate::download::disk::DiskIoActor::new(
                std::path::PathBuf::from(downloads_dir),
                disk_cmd_rx,
                disk_event_tx,
            );
            tokio::spawn(disk_actor.run());

            // 1. Kick off concurrent UDP announces
            TrackerScraper::start(parsed.trackers, raw_info_hash, peer_id, ip_tx.clone());

            crate::tracker::dht_scraper::DhtScraper::start(raw_info_hash, ip_tx);

            // 2. Start the core state machine for this torrent
            let manager = TorrentManagerActor::new(
                raw_info_hash,
                info_hash_hex,
                event_tx,
                event_rx,
                ip_rx,
                incoming_rx,
                disk_cmd_tx,
                disk_event_rx,
                shared_state,
                download_speed_limit,
                db_pool,
                "PENDING_METADATA".to_string(),
            );

            manager.run().await;
        });

        Ok(())
    }

    #[napi]
    pub fn open_downloads_folder(&self) -> napi::Result<()> {
        let dl_dir = self.downloads_dir.clone();
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(&dl_dir).spawn();
        }
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("explorer").arg(&dl_dir).spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open").arg(&dl_dir).spawn();
        }
        Ok(())
    }

    #[napi]
    pub fn pause_torrent(&self, info_hash: String) -> napi::Result<()> {
        if let Ok(mut st) = self.shared_state.write()
            && let Some(t) = st.torrents.get_mut(&info_hash) {
                t.state_string = "PAUSED".to_string();
            }
        let pool = self.db_pool.clone();
        let hash = info_hash.clone();
        self.rt.spawn(async move {
            let _ = crate::db::update_session_state(&pool, &hash, "PAUSED").await;
        });
        Ok(())
    }

    #[napi]
    pub fn resume_torrent(&self, info_hash: String) -> napi::Result<()> {
        let uri = {
            let st = self.shared_state.read().unwrap();
            st.torrents.get(&info_hash).map(|t| t.magnet_uri.clone())
        };

        if let Some(magnet_uri) = uri {
            self.add_magnet(magnet_uri, None)?;
        }
        Ok(())
    }

    #[napi]
    pub fn cancel_torrent(&self, info_hash: String) -> napi::Result<()> {
        // 1. Get the torrent name to delete files
        let mut torrent_name = None;
        if let Ok(mut st) = self.shared_state.write() {
            if let Some(t) = st.torrents.get(&info_hash) {
                torrent_name = Some(t.torrent_name.clone());
            }
            // Remove from in-memory state so it immediately disappears from UI
            st.torrents.remove(&info_hash);
        }

        let pool = self.db_pool.clone();
        let downloads_dir = self.downloads_dir.clone();

        self.rt.spawn(async move {
            // 2. Remove from Database
            let _ = sqlx::query("DELETE FROM torrent_sessions WHERE info_hash = ?")
                .bind(&info_hash)
                .execute(&pool)
                .await;

            // 3. Delete partially downloaded files from disk
            if let Some(name) = torrent_name
                && !name.starts_with("Magnet:")
            {
                let path = std::path::PathBuf::from(downloads_dir).join(name);
                if path.exists() {
                    if path.is_dir() {
                        let _ = std::fs::remove_dir_all(path);
                    } else {
                        let _ = std::fs::remove_file(path);
                    }
                }
            }
        });

        Ok(())
    }

    #[napi(constructor)]
    pub fn new(profile_name: String) -> napi::Result<Self> {
        // 1. Cross-platform config dirs
        let proj_dirs = ProjectDirs::from("", "", "torry").ok_or_else(|| {
            napi::Error::from_reason("Failed to resolve system config directories")
        })?;

        let mut root_dir = proj_dirs.config_dir().to_path_buf();
        root_dir.push("profiles");
        root_dir.push(&profile_name);

        fs::create_dir_all(&root_dir).map_err(|e| {
            napi::Error::from_reason(format!("Failed to create profile dir: {}", e))
        })?;

        // 2. Lockfile Architecture (Prevent dual-instance corruption)
        let lock_path = root_dir.join("profile.lock");
        let lock_file = File::create(&lock_path)
            .map_err(|e| napi::Error::from_reason(format!("Failed to create lock file: {}", e)))?;

        // Exclusive non-blocking lock. Fails immediately if another process holds it.
        lock_file.try_lock_exclusive().map_err(|_| {
            napi::Error::from_reason(format!(
                "Profile '{}' is already running in another process.",
                profile_name
            ))
        })?;

        // 3. Initialize Subsystems
        let db_path = root_dir.join("db.sqlite");
        let torrents_dir = root_dir.join("torrents");
        let downloads_dir = UserDirs::new()
            .and_then(|u| u.download_dir().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| root_dir.join("downloads"));
        let cache_dir = root_dir.join("cache");

        // Ensure all foundational directories exist before the app boots
        fs::create_dir_all(&torrents_dir).ok();
        fs::create_dir_all(&downloads_dir).ok();
        fs::create_dir_all(&cache_dir).ok();

        // 4. Initialize Tokio Runtime
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| {
                napi::Error::from_reason(format!("Failed to build Tokio runtime: {}", e))
            })?;

        let db_path_str = db_path.to_string_lossy().to_string();
        let db_pool = rt
            .block_on(crate::db::init_db(&db_path_str))
            .map_err(|e| napi::Error::from_reason(format!("Failed to init DB: {}", e)))?;

        let download_speed_limit = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));

        let shared_state = Arc::new(RwLock::new(crate::state::EngineSnapshot {
            torrents: std::collections::HashMap::new(),
        }));

        let incoming_routers: Arc<RwLock<std::collections::HashMap<[u8; 20], mpsc::Sender<(tokio::net::TcpStream, [u8; 8], [u8; 20])>>>> = 
            Arc::new(RwLock::new(std::collections::HashMap::new()));

        // Spawn global TCP Listen Server on port 6881 for incoming DHT/PEX peers
        let routers_clone = incoming_routers.clone();
        rt.spawn(async move {
            use tokio::net::TcpListener;
            use tokio::io::AsyncReadExt;
            
            let listener = match TcpListener::bind("0.0.0.0:6881").await {
                Ok(l) => l,
                Err(_) => {
                    // Fallback to dynamic port if 6881 is busy
                    match TcpListener::bind("0.0.0.0:0").await {
                        Ok(l) => l,
                        Err(_e) => {
                            // eprintln!("[ListenServer] Failed to bind: {}", e);
                            return;
                        }
                    }
                }
            };
            
            // println!("[ListenServer] Bound to {:?}", listener.local_addr());

            loop {
                if let Ok((mut socket, _addr)) = listener.accept().await {
                    let routers = routers_clone.clone();
                    tokio::spawn(async move {
                        // Read BitTorrent handshake (68 bytes) with timeout
                        let mut handshake = [0u8; 68];
                        if let Ok(Ok(_)) = tokio::time::timeout(
                            std::time::Duration::from_secs(5),
                            socket.read_exact(&mut handshake)
                        ).await {
                            // Validate protocol identifier
                            if &handshake[0..20] == b"\x13BitTorrent protocol" {
                                let mut reserved = [0u8; 8];
                                reserved.copy_from_slice(&handshake[20..28]);
                                let mut info_hash = [0u8; 20];
                                info_hash.copy_from_slice(&handshake[28..48]);
                                let mut peer_id = [0u8; 20];
                                peer_id.copy_from_slice(&handshake[48..68]);
                                
                                // Route to the correct torrent manager
                                let tx = {
                                    let map = routers.read().unwrap();
                                    map.get(&info_hash).cloned()
                                };
                                
                                if let Some(tx) = tx {
                                    let _ = tx.send((socket, reserved, peer_id)).await;
                                }
                            }
                        }
                    });
                }
            }
        });

        use sqlx::Row;
        let sessions: Vec<crate::session::TorrentSession> = rt.block_on(async {
        sqlx::query("SELECT info_hash, display_name, magnet_uri, state, added_at, source FROM torrent_sessions")
            .fetch_all(&db_pool)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|row| {
                let state_str: String = row.get("state");
                let state = match state_str.as_str() {
                    "PENDING_METADATA" => crate::session::SessionState::PendingMetadata,
                    "FETCHING_METADATA" => crate::session::SessionState::FetchingMetadata,
                    "METADATA_RESOLVED" => crate::session::SessionState::MetadataResolved,
                    "DOWNLOADING" => crate::session::SessionState::Downloading,
                    "SEEDING" => crate::session::SessionState::Seeding,
                    "COMPLETED" => crate::session::SessionState::Completed,
                    "PAUSED" => crate::session::SessionState::Paused,
                    "ERROR" => crate::session::SessionState::Error,
                    "ERROR_MISSING_FILES" => crate::session::SessionState::ErrorMissingFiles,
                    _ => crate::session::SessionState::PendingMetadata,
                };
                let source_opt: Option<String> = row.try_get("source").unwrap_or(None);
                crate::session::TorrentSession {
                    info_hash: row.get("info_hash"),
                    display_name: row.get("display_name"),
                    magnet_uri: row.get("magnet_uri"),
                    source: source_opt,
                    state,
                    added_at: row.get("added_at"),
                }
            })
            .collect::<Vec<_>>()
    });

        let dl_dir = downloads_dir.clone();
        for session in sessions {
            let uri = session.magnet_uri.clone();
            if let Ok(parsed) = crate::magnet::parse_magnet_uri(uri.clone())
                && let Ok(raw_info_hash) = decode_hex(&parsed.info_hash)
            {
                let peer_id: [u8; 20] = rand::random();
                let dl_dir_str = dl_dir.to_string_lossy().to_string();
                let st = shared_state.clone();
                let info_hash_hex = parsed.info_hash.clone();

                if let Ok(mut lock) = shared_state.write() {
                    lock.torrents.insert(
                        info_hash_hex.clone(),
                        crate::state::TorrentSnapshot {
                            info_hash: info_hash_hex.clone(),
                            torrent_name: parsed
                                .display_name
                                .clone()
                                .unwrap_or_else(|| format!("Magnet: {}", info_hash_hex)),
                            magnet_uri: uri.clone(),
                            state_string: if session.state == crate::session::SessionState::Paused { "PAUSED".to_string() } else if session.state == crate::session::SessionState::ErrorMissingFiles { "ERROR_MISSING_FILES".to_string() } else { "FETCHING_METADATA".to_string() },
                            source: session
                                .source
                                .clone()
                                .unwrap_or_else(|| "Unknown".to_string()),
                            total_bytes: 0.0,
                            ..Default::default()
                        },
                    );
                }

                let (incoming_tx, incoming_rx) = mpsc::channel(100);
                {
                    let mut routers = incoming_routers.write().unwrap();
                    routers.insert(raw_info_hash, incoming_tx);
                }

                let speed_limit_clone = download_speed_limit.clone();
                let pool_clone = db_pool.clone();
                rt.spawn(async move {
                    let (ip_tx, ip_rx) = mpsc::channel(4096);
                    let (event_tx, event_rx) = mpsc::channel(16384);

                    let (disk_cmd_tx, disk_cmd_rx) = mpsc::channel(8192);
                    let (disk_event_tx, disk_event_rx) = mpsc::channel(8192);

                    let disk_actor = crate::download::disk::DiskIoActor::new(
                        std::path::PathBuf::from(dl_dir_str),
                        disk_cmd_rx,
                        disk_event_tx,
                    );
                    tokio::spawn(disk_actor.run());

                    crate::tracker::scraper::TrackerScraper::start(
                        parsed.trackers,
                        raw_info_hash,
                        peer_id,
                        ip_tx.clone(),
                    );

                    crate::tracker::dht_scraper::DhtScraper::start(raw_info_hash, ip_tx);

                    let initial_state = match session.state {
                        crate::session::SessionState::Completed => "COMPLETED".to_string(),
                        crate::session::SessionState::Seeding => "SEEDING".to_string(),
                        crate::session::SessionState::Paused => "PAUSED".to_string(),
                        crate::session::SessionState::ErrorMissingFiles => "ERROR_MISSING_FILES".to_string(),
                        _ => "DOWNLOADING".to_string(),
                    };

                    let manager = crate::download::manager::TorrentManagerActor::new(
                        raw_info_hash,
                        info_hash_hex,
                        event_tx,
                        event_rx,
                        ip_rx,
                        incoming_rx,
                        disk_cmd_tx,
                        disk_event_rx,
                        st,
                        speed_limit_clone,
                        pool_clone,
                        initial_state,
                    );

                    manager.run().await;
                });
            }
        }

        Ok(Self {
            name: profile_name,
            db_path: db_path_str,
            torrents_dir: torrents_dir.to_string_lossy().to_string(),
            downloads_dir: downloads_dir.to_string_lossy().to_string(),
            cache_dir: cache_dir.to_string_lossy().to_string(),
            lock: Some(lock_file),
            rt,
            db_pool,
            shared_state,
            incoming_routers,
            download_speed_limit,
        })
    }
}

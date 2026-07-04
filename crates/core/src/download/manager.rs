use crate::peer::actor::{PeerCommand, PeerEvent};
use std::collections::{HashMap, HashSet, VecDeque};
use std::net::SocketAddr;
use tokio::sync::mpsc;

#[derive(Clone, Default)]
pub struct SwarmState {
    pub piece_to_peers: HashMap<u32, HashSet<SocketAddr>>,
    pub peer_to_pieces: HashMap<SocketAddr, HashSet<u32>>,
    pub piece_rarity: HashMap<u32, usize>,
    pub peers: HashSet<SocketAddr>,
}

pub struct SwarmView<'a> {
    state: &'a SwarmState,
}

impl<'a> SwarmView<'a> {
    pub fn new(state: &'a SwarmState) -> Self {
        Self { state }
    }

    pub fn piece_rarity(&self, piece: u32) -> usize {
        self.state.piece_rarity.get(&piece).copied().unwrap_or(0)
    }

    pub fn peers_with_piece(&self, piece: u32) -> Option<&HashSet<SocketAddr>> {
        self.state.piece_to_peers.get(&piece)
    }

    pub fn peer_pieces(&self, peer: &SocketAddr) -> Option<&HashSet<u32>> {
        self.state.peer_to_pieces.get(peer)
    }

    pub fn rarest_pieces(&self) -> Vec<u32> {
        let mut pieces: Vec<u32> = self.state.piece_rarity.keys().copied().collect();
        pieces.sort_by_key(|p| self.state.piece_rarity.get(p).unwrap());
        pieces
    }
}

const MAX_ACTIVE_PEERS: usize = 50;

#[derive(Debug, Clone)]
pub struct PeerTelemetry {
    pub connected_at: std::time::Instant,
    pub bytes_downloaded: u64,
    pub throughput_ema: f64,
    pub last_tick_bytes: u64,
    pub latency_ema_ms: f64,
    pub latency_samples: u32,
    pub reconnect_count: u32,
}

impl Default for PeerTelemetry {
    fn default() -> Self {
        Self {
            connected_at: std::time::Instant::now(),
            bytes_downloaded: 0,
            throughput_ema: 0.0,
            last_tick_bytes: 0,
            latency_ema_ms: 0.0,
            latency_samples: 0,
            reconnect_count: 0,
        }
    }
}

pub struct PeerFeatureVector {
    pub uptime: std::time::Duration,
    pub is_choked: bool,
    pub missing_piece_overlap: u32,
    pub rare_piece_count: u32,
    pub throughput_ema: f64,
    pub avg_request_latency_ms: f64,
    pub connection_stability: f64,
}

/// The central brain for a single torrent download.
/// This actor orchestrates many PeerActors without ever touching a TCP socket directly.
pub struct TorrentManagerActor {
    info_hash: [u8; 20],
    info_hash_hex: String,

    // Active connections mapped by IP
    peer_channels: HashMap<SocketAddr, mpsc::Sender<PeerCommand>>,

    // The strict Peer Pool Limiter waiting room
    queued_ips: VecDeque<SocketAddr>,

    // Inbox for active peers sending us data
    event_rx: mpsc::Receiver<(SocketAddr, PeerEvent)>,

    // The sender side to clone for new peers
    event_tx: mpsc::Sender<(SocketAddr, PeerEvent)>,

    // Inbox for the TrackerScraper finding new IPs
    ip_rx: mpsc::Receiver<SocketAddr>,
    
    // Inbox for incoming TCP connections from Listen Server
    incoming_rx: mpsc::Receiver<(tokio::net::TcpStream, [u8; 8], [u8; 20])>,

    // BEP 9 Metadata Assembly State
    metadata_size: Option<u32>,
    metadata_buffer: Vec<u8>,
    metadata_pieces_received: HashSet<u32>,
    metadata_pieces_requested: HashSet<u32>,
    metadata_complete: bool,
    // Disk IO Actor communication
    disk_tx: mpsc::Sender<crate::download::disk::DiskCommand>,
    disk_rx: mpsc::Receiver<crate::download::disk::DiskEvent>,

    // BEP 3 Downloading State
    torrent_info: Option<crate::metadata::TorrentInfo>,
    files_allocated: bool,
    peer_bitfields: HashMap<SocketAddr, HashSet<u32>>,
    unchoked_peers: HashSet<SocketAddr>,
    peers_we_unchoked: HashSet<SocketAddr>,
    peers_interested_in_us: HashSet<SocketAddr>,
    last_choke_calc: std::time::Instant,
    downloaded_pieces: HashSet<u32>,
    requested_blocks: HashMap<(u32, u32), Vec<(SocketAddr, std::time::Instant)>>, // (piece, offset) -> requesters
    active_piece_buffers: HashMap<u32, Vec<u8>>, // piece_index -> full piece buffer in RAM
    blocks_received: HashMap<u32, HashSet<u32>>, // piece_index -> set of block offsets
    peer_in_flight: HashMap<SocketAddr, u32>,
    peer_telemetry: HashMap<SocketAddr, PeerTelemetry>, // Replaces separate bytes and time trackers

    // Shared state to report to TUI
    shared_state: std::sync::Arc<std::sync::RwLock<crate::state::EngineSnapshot>>,

    // The Swarm Graph Engine State
    swarm_state_mut: SwarmState, // Primary write target
    cached_rarest_pieces: Vec<u32>,
    last_publish_time: std::time::Instant,
    event_count_since_publish: u32,

    last_request_time: std::time::Instant,

    // Speed Limiting (Token Bucket)
    download_speed_limit: std::sync::Arc<std::sync::atomic::AtomicU64>, // bytes/sec, 0 = unlimited
    token_bucket_bytes: f64,
    last_token_refill: std::time::Instant,

    db_pool: sqlx::SqlitePool,
    initial_state: String,
}

impl TorrentManagerActor {
    pub fn new(
        info_hash: [u8; 20],
        info_hash_hex: String,
        event_tx: mpsc::Sender<(SocketAddr, PeerEvent)>,
        event_rx: mpsc::Receiver<(SocketAddr, PeerEvent)>,
        ip_rx: mpsc::Receiver<SocketAddr>,
        incoming_rx: mpsc::Receiver<(tokio::net::TcpStream, [u8; 8], [u8; 20])>,
        disk_tx: mpsc::Sender<crate::download::disk::DiskCommand>,
        disk_rx: mpsc::Receiver<crate::download::disk::DiskEvent>,
        shared_state: std::sync::Arc<std::sync::RwLock<crate::state::EngineSnapshot>>,
        download_speed_limit: std::sync::Arc<std::sync::atomic::AtomicU64>,
        db_pool: sqlx::SqlitePool,
        initial_state: String,
    ) -> Self {
        Self {
            info_hash,
            info_hash_hex,
            peer_channels: HashMap::new(),
            queued_ips: VecDeque::new(),
            event_rx,
            event_tx,
            ip_rx,
            incoming_rx,
            metadata_size: None,
            metadata_buffer: Vec::new(),
            metadata_pieces_received: HashSet::new(),
            metadata_pieces_requested: HashSet::new(),
            metadata_complete: false,
            disk_tx,
            disk_rx,
            torrent_info: None,
            files_allocated: false,
            peer_bitfields: HashMap::new(),
            unchoked_peers: HashSet::new(),
            peers_we_unchoked: HashSet::new(),
            peers_interested_in_us: HashSet::new(),
            last_choke_calc: std::time::Instant::now(),
            downloaded_pieces: HashSet::new(),
            requested_blocks: HashMap::new(),
            active_piece_buffers: HashMap::new(),
            blocks_received: HashMap::new(),
            peer_in_flight: HashMap::new(),
            peer_telemetry: HashMap::new(),
            shared_state,
            swarm_state_mut: SwarmState::default(),
            cached_rarest_pieces: Vec::new(),
            last_publish_time: std::time::Instant::now(),
            event_count_since_publish: 0,

            last_request_time: std::time::Instant::now(),
            download_speed_limit,
            token_bucket_bytes: 0.0,
            last_token_refill: std::time::Instant::now(),
            db_pool,
            initial_state,
        }
    }

    fn update_state(&self, new_state: &str) {
        if let Ok(mut st) = self.shared_state.write()
            && let Some(ts) = st.torrents.get_mut(&self.info_hash_hex) {
                ts.state_string = new_state.to_string();
            }
        let pool = self.db_pool.clone();
        let info_hash = self.info_hash_hex.clone();
        let state = new_state.to_string();
        tokio::spawn(async move {
            let _ = crate::db::update_session_state(&pool, &info_hash, &state).await;
        });
    }

    /// Spawns a new PeerActor only if we have capacity in the pool.
    fn try_spawn_peer(&mut self, ip: SocketAddr) {
        if self.peer_channels.len() >= MAX_ACTIVE_PEERS {
            // Pool is full, push to the waiting room silently
            if !self.queued_ips.contains(&ip) && !self.peer_channels.contains_key(&ip) {
                self.queued_ips.push_back(ip);
            }
            return;
        }

        // We have capacity. Spawn the actor.
        let (cmd_tx, cmd_rx) = mpsc::channel(2048);
        self.peer_channels.insert(ip, cmd_tx);
        self.peer_telemetry
            .entry(ip)
            .and_modify(|t| t.reconnect_count += 1)
            .or_default();

        let peer_id: [u8; 20] = rand::random(); // MVP random ID per connection or should be global? Global is better but MVP is fine.
        let tx = self.event_tx.clone();

        let actor = crate::peer::actor::PeerActor::new(ip, self.info_hash, peer_id, tx, cmd_rx);

        tokio::spawn(actor.run());
        self.update_peers_snapshot();
    }

    fn update_peers_snapshot(&self) {
        if let Ok(mut st) = self.shared_state.write() {
            let mut peers = Vec::new();
            for ip in self.peer_channels.keys() {
                peers.push(crate::state::PeerStateSnapshot {
                    ip: ip.to_string(),
                    unchoked: self.unchoked_peers.contains(ip),
                    blocks_in_flight: self.peer_in_flight.get(ip).copied().unwrap_or(0),
                });
            }
            if let Some(ts) = st.torrents.get_mut(&self.info_hash_hex) {
                ts.peers = peers;
                ts.active_peers = self.peer_channels.len() as u32;
            }
        }
    }

    fn maybe_update_rarest_pieces(&mut self) {
        self.event_count_since_publish += 1;
        if self.event_count_since_publish.is_multiple_of(100)
            || self.last_publish_time.elapsed().as_millis() > 200
        {
            let mut pieces: Vec<u32> = self.swarm_state_mut.piece_rarity.keys().copied().collect();
            pieces.sort_by_key(|p| self.swarm_state_mut.piece_rarity.get(p).unwrap_or(&0));
            self.cached_rarest_pieces = pieces;

            self.last_publish_time = std::time::Instant::now();
            self.event_count_since_publish = 0;
        }
    }

    fn extract_peer_features(
        &self,
        addr: SocketAddr,
        telemetry: &PeerTelemetry,
        view: &SwarmView,
    ) -> PeerFeatureVector {
        let uptime = std::time::Instant::now().duration_since(telemetry.connected_at);
        let is_choked = !self.unchoked_peers.contains(&addr);
        // Lower reconnect count is more stable.
        let connection_stability = 1.0 / (1.0 + telemetry.reconnect_count as f64);

        let mut missing_piece_overlap = 0;
        let mut rare_piece_count = 0;

        if let Some(bitfield) = view.peer_pieces(&addr) {
            for &piece_index in bitfield.iter() {
                if !self.downloaded_pieces.contains(&piece_index) {
                    missing_piece_overlap += 1;
                }
                let rarity = view.piece_rarity(piece_index);
                if rarity > 0 && rarity <= 2 {
                    rare_piece_count += 1;
                }
            }
        }

        PeerFeatureVector {
            uptime,
            is_choked,
            missing_piece_overlap,
            rare_piece_count,
            throughput_ema: telemetry.throughput_ema,
            avg_request_latency_ms: telemetry.latency_ema_ms,
            connection_stability,
        }
    }

    fn prune_useless_peers(&mut self) {
        if self.queued_ips.is_empty() {
            return; // No one is waiting in line, keep everyone
        }

        let _now = std::time::Instant::now();
        let mut legacy_drop_set = HashSet::new();
        let mut view_drop_set = HashSet::new();
        let mut hard_protected = HashSet::new();

        let view = SwarmView::new(&self.swarm_state_mut);

        for (addr, telemetry) in &self.peer_telemetry {
            let features = self.extract_peer_features(*addr, telemetry, &view);
            let mut legacy_decision = false;
            let mut legacy_reason = "KEEP";

            // Give peers at least 45 seconds to prove their worth
            if features.uptime.as_secs() < 45 {
                hard_protected.insert(*addr);
                legacy_reason = "PROTECT_UPTIME";
            } else if features.rare_piece_count > 0 {
                // HARD PROTECTION FILTER
                hard_protected.insert(*addr);
                legacy_reason = "PROTECT_RARE";
            } else {
                // ==========================================
                // LEGACY ORACLE (O(N) operations allowed here)
                // ==========================================
                if features.is_choked && telemetry.bytes_downloaded == 0 {
                    legacy_drop_set.insert(*addr);
                    legacy_decision = true;
                    legacy_reason = "DROP_CHOKED_ZERO_BYTES";
                } else if let Some(bitfield) = self.peer_bitfields.get(addr) {
                    let has_useful = bitfield
                        .iter()
                        .any(|idx| !self.downloaded_pieces.contains(idx));
                    if !has_useful {
                        legacy_drop_set.insert(*addr);
                        legacy_decision = true;
                        legacy_reason = "DROP_NO_USEFUL_PIECES";
                    }
                }

                // ==========================================
                // VIEW ORACLE (O(1) graph lookups)
                // ==========================================
                if features.is_choked && telemetry.bytes_downloaded == 0 {
                    view_drop_set.insert(*addr);
                } else if let Some(bitfield) = view.peer_pieces(addr) {
                    let has_useful = bitfield
                        .iter()
                        .any(|idx| !self.downloaded_pieces.contains(idx));
                    if !has_useful {
                        view_drop_set.insert(*addr);
                    }
                }
            }

            // Log telemetry occasionally or when dropped, to avoid flooding stdout every 2s for 50 peers
            // We can just log if it's a drop, or randomly 1/100 of the time for keeps.
            if legacy_decision || rand::random::<u8>() < 5 {
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis();

                let log_str = format!(
                    r#"{{"timestamp":{},"context":"prune_useless_peers","peer":"{}","legacy_drop":{},"reason":"{}","features":{{"uptime_s":{},"choked":{},"missing_overlap":{},"rare_count":{},"tput_ema":{:.1},"lat_ema":{:.1},"stability":{:.2}}}}}"#,
                    timestamp,
                    addr,
                    legacy_decision,
                    legacy_reason,
                    features.uptime.as_secs(),
                    features.is_choked,
                    features.missing_piece_overlap,
                    features.rare_piece_count,
                    features.throughput_ema,
                    features.avg_request_latency_ms,
                    features.connection_stability
                );

                // Write to telemetry.jsonl in the current directory asynchronously
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("telemetry.jsonl")
                {
                    use std::io::Write;
                    let _ = writeln!(file, "{}", log_str);
                }
            }
        }

        // ==========================================
        // STAGE 1: SHADOW PRUNING & TELEMETRY
        // ==========================================
        if legacy_drop_set != view_drop_set {
            // Silencing divergence logs for TUI stability
            // eprintln!("[Manager] ⚠️ PRUNING DIVERGENCE! Legacy wants: {:?}, View wants: {:?}", legacy_drop_set, view_drop_set);
        }

        // Conservative Intersection + Hard Protection
        let mut actual_drop: Vec<SocketAddr> = legacy_drop_set
            .intersection(&view_drop_set)
            .copied()
            .collect();
        actual_drop.retain(|addr| !hard_protected.contains(addr));

        // Churn Rate Limiter (Max 3 peer churns per cycle)
        actual_drop.truncate(3);

        for addr in actual_drop {
            if let Some(tx) = self.peer_channels.get(&addr) {
                let _ = tx.try_send(PeerCommand::Disconnect);
            }
        }
    }

    fn try_request_blocks(&mut self) {
        if !self.files_allocated {
            return;
        }

        // Check if we are paused or in an error state
        if let Ok(st) = self.shared_state.read()
            && let Some(ts) = st.torrents.get(&self.info_hash_hex)
                && (ts.state_string == "PAUSED" || ts.state_string == "ERROR_MISSING_FILES") {
                    return;
                }

        // Speed Limit: Token Bucket Enforcement
        let limit = self
            .download_speed_limit
            .load(std::sync::atomic::Ordering::Relaxed);
        if limit > 0 {
            let now_tb = std::time::Instant::now();
            let elapsed = now_tb.duration_since(self.last_token_refill).as_secs_f64();
            self.last_token_refill = now_tb;
            self.token_bucket_bytes += elapsed * limit as f64;
            let max_bucket = limit as f64 * 2.0;
            if self.token_bucket_bytes > max_bucket {
                self.token_bucket_bytes = max_bucket;
            }
            if self.token_bucket_bytes < 16384.0 {
                return;
            }
        }

        let info = if let Some(ref i) = self.torrent_info {
            i
        } else {
            return;
        };

        let total_pieces = info.pieces.len() as u32;
        let piece_length = info.piece_length;

        // 2. Sort peers by their download speed/reliability (fastest peers get first pick of rare pieces)
        let mut peers: Vec<SocketAddr> = self.unchoked_peers.iter().cloned().collect();
        peers.sort_by_key(|addr| {
            std::cmp::Reverse(
                self.peer_telemetry
                    .get(addr)
                    .map(|t| t.bytes_downloaded)
                    .unwrap_or(0),
            )
        });

        for peer_addr in peers {
            let mut target_in_flight = if let Some(tel) = self.peer_telemetry.get(&peer_addr) {
                if tel.throughput_ema > 0.0 && tel.latency_ema_ms > 0.0 {
                    // Bandwidth-Delay Product (BDP) in bytes = throughput * latency
                    // Add a 8.0x multiplier to ensure pipe never runs dry, max out at 2048 blocks (~33MB)
                    let bdp_blocks = ((tel.throughput_ema * (tel.latency_ema_ms / 1000.0) * 8.0)
                        / 16384.0)
                        .ceil() as u32;
                    bdp_blocks.clamp(200, 2048)
                } else {
                    200 // Default baseline for new peers before telemetry stabilizes
                }
            } else {
                200
            };
            target_in_flight = std::cmp::min(target_in_flight, 2048);
            let in_flight = self.peer_in_flight.entry(peer_addr).or_insert(0);
            if *in_flight >= target_in_flight {
                continue;
            } // Dynamic BDP limit

            let peer_pieces = self.swarm_state_mut.peer_to_pieces.get(&peer_addr);

            let mut pieces_to_request = Vec::new();
            if let Some(bitfield) = peer_pieces {
                // 1. Prioritize pieces we are already actively downloading
                for &piece in self.active_piece_buffers.keys() {
                    if bitfield.contains(&piece) {
                        pieces_to_request.push(piece);
                    }
                }

                // 2. Add pieces according to Piece Picking Algorithm (Random-First then Rarest-First)
                let mut candidate_pieces = Vec::new();

                if self.downloaded_pieces.len() < 4 {
                    // Random-First for fast initial startup
                    let available: Vec<u32> = bitfield.iter()
                        .filter(|&p| !self.downloaded_pieces.contains(p) && !self.active_piece_buffers.contains_key(p))
                        .copied()
                        .collect();
                    
                    if !available.is_empty() {
                        use rand::seq::SliceRandom;
                        let mut rng = rand::thread_rng();
                        let count = std::cmp::min(available.len(), 500);
                        let mut selected: Vec<u32> = available.choose_multiple(&mut rng, count).copied().collect();
                        candidate_pieces.append(&mut selected);
                    }
                } else {
                    // Rarest-First for swarm health and piece availability
                    for &piece in &self.cached_rarest_pieces {
                        if candidate_pieces.len() >= 500 {
                            break; // Scan enough pieces to saturate pipeline (up to 2048 blocks)
                        }
                        if bitfield.contains(&piece)
                            && !self.downloaded_pieces.contains(&piece)
                            && !self.active_piece_buffers.contains_key(&piece)
                        {
                            candidate_pieces.push(piece);
                        }
                    }
                }

                pieces_to_request.extend(candidate_pieces);
            }

            for piece_index in pieces_to_request {
                if *in_flight >= target_in_flight {
                    break;
                } // Dynamic BDP limit
                let mut block_offset = 0;
                let mut piece_size = piece_length;
                if piece_index == total_pieces - 1 {
                    let rem = (info.total_size % piece_length as u64) as u32;
                    if rem > 0 {
                        piece_size = rem;
                    }
                }

                while block_offset < piece_size && *in_flight < target_in_flight {
                    let block_len = std::cmp::min(16384, piece_size - block_offset);

                    let is_already_downloaded = self
                        .blocks_received
                        .get(&piece_index)
                        .is_some_and(|blocks| blocks.contains(&block_offset));

                    if is_already_downloaded {
                        block_offset += block_len;
                        continue;
                    }

                    let mut should_request = false;

                    // Check completion percentage
                    let progress = self.downloaded_pieces.len() as f64 / total_pieces as f64;
                    let is_endgame = progress >= 0.95;

                    let requesters = self
                        .requested_blocks
                        .entry((piece_index, block_offset))
                        .or_default();

                    if requesters.is_empty() {
                        should_request = true;
                    } else if is_endgame
                        && requesters.len() < 4
                        && !requesters.iter().any(|(addr, _)| *addr == peer_addr)
                    {
                        // Redundant Swarm Blasting Mode!
                        should_request = true;
                    }

                    if should_request {
                        requesters.push((peer_addr, std::time::Instant::now()));
                        *in_flight += 1;

                        if let Some(tx) = self.peer_channels.get(&peer_addr) {
                            let _ = tx.try_send(PeerCommand::RequestPiece {
                                index: piece_index,
                                offset: block_offset,
                                length: block_len,
                            });
                        }
                    }
                    block_offset += block_len;
                }
            }
        }
    }

    /// The central orchestration loop.
    pub async fn run(mut self) {
        let mut timeout_interval = tokio::time::interval(std::time::Duration::from_secs(2));

        // Because peers handle their own TCP parsing on separate cores,
        // this loop is incredibly fast and non-blocking.
        loop {
            tokio::select! {
                _ = timeout_interval.tick() => {
                    let now = std::time::Instant::now();
                    
                    // Check if we were cancelled (removed from shared_state by user) or paused
                    let mut exists = false;
                    let mut is_paused = false;
                    if let Ok(st) = self.shared_state.read()
                        && let Some(ts) = st.torrents.get(&self.info_hash_hex) {
                            exists = true;
                            is_paused = ts.state_string == "PAUSED";
                        }
                    if !exists || is_paused {
                        // Actor was cancelled or paused!
                        break;
                    }

                    // Update tick-based telemetry (Throughput EMA)
                    let mut total_current_bps = 0.0;
                    for tel in self.peer_telemetry.values_mut() {
                        let bytes_since_last_tick = tel.bytes_downloaded.saturating_sub(tel.last_tick_bytes);
                        tel.last_tick_bytes = tel.bytes_downloaded;
                        let current_bps = (bytes_since_last_tick as f64) / 2.0; // 2 sec tick
                        tel.throughput_ema = (current_bps * 0.2) + (tel.throughput_ema * 0.8);
                        total_current_bps += current_bps;
                    }

                    if let Ok(mut st) = self.shared_state.write()
                        && let Some(ts) = st.torrents.get_mut(&self.info_hash_hex) {
                            // Smooth the global download speed slightly to avoid UI flicker
                            ts.download_speed = (total_current_bps * 0.5) + (ts.download_speed * 0.5);
                            let limit = self.download_speed_limit.load(std::sync::atomic::Ordering::Relaxed);
                            ts.download_speed_limit = limit as f64;
                        }

                    if now.duration_since(self.last_choke_calc).as_secs() >= 10 {
                        self.last_choke_calc = now;

                        let mut interested: Vec<_> = self.peers_interested_in_us.iter().copied().collect();
                        interested.sort_by(|a, b| {
                            let speed_a = self.peer_telemetry.get(a).map(|t| t.throughput_ema).unwrap_or(0.0);
                            let speed_b = self.peer_telemetry.get(b).map(|t| t.throughput_ema).unwrap_or(0.0);
                            speed_b.partial_cmp(&speed_a).unwrap_or(std::cmp::Ordering::Equal)
                        });

                        let unchoke_count = std::cmp::min(3, interested.len());
                        let mut new_unchoked: HashSet<SocketAddr> = interested.iter().take(unchoke_count).copied().collect();

                        if interested.len() > 3 {
                            use rand::seq::SliceRandom;
                            if let Some(&optimistic) = interested[3..].choose(&mut rand::thread_rng()) {
                                new_unchoked.insert(optimistic);
                            }
                        }

                        for &peer in &self.peers_we_unchoked {
                            if !new_unchoked.contains(&peer)
                                && let Some(tx) = self.peer_channels.get(&peer) {
                                    let _ = tx.try_send(PeerCommand::Choke);
                                }
                        }
                        for &peer in &new_unchoked {
                            if !self.peers_we_unchoked.contains(&peer)
                                && let Some(tx) = self.peer_channels.get(&peer) {
                                    let _ = tx.try_send(PeerCommand::Unchoke);
                                }
                        }
                        self.peers_we_unchoked = new_unchoked;
                    }


                    for requesters in self.requested_blocks.values_mut() {
                        requesters.retain(|(addr, timestamp)| {
                            if now.duration_since(*timestamp).as_secs() > 10 {
                                // Block timed out, decrement in-flight for this peer so we can request something else
                                if let Some(in_flight) = self.peer_in_flight.get_mut(addr)
                                    && *in_flight > 0 { *in_flight -= 1; }
                                false
                            } else {
                                true
                            }
                        });
                    }
                    self.requested_blocks.retain(|_, reqs| !reqs.is_empty());
                    self.prune_useless_peers();
                    self.update_peers_snapshot();
                    // Force try_request_blocks by resetting throttle timer
                    self.last_request_time = std::time::Instant::now() - std::time::Duration::from_secs(1);
                    self.try_request_blocks();
                }

                // 1. Intake new IPs streaming in from the TrackerScraper
                Some(new_ip) = self.ip_rx.recv() => {
                    self.try_spawn_peer(new_ip);
                }

                // 1.b Intake new TCP Streams from Listen Server
                Some((socket, reserved_bytes, their_peer_id)) = self.incoming_rx.recv() => {
                    let addr = socket.peer_addr().unwrap_or_else(|_| "0.0.0.0:0".parse().unwrap());
                    if !self.peer_channels.contains_key(&addr) && self.peer_channels.len() < MAX_ACTIVE_PEERS {
                        let (cmd_tx, cmd_rx) = mpsc::channel(2048);
                        self.peer_channels.insert(addr, cmd_tx);
                        let event_tx = self.event_tx.clone();
                        let info_hash = self.info_hash;
                        let peer_id: [u8; 20] = rand::random();
                        tokio::spawn(async move {
                            let stream = crate::peer::stream::PeerStream::Tcp(socket);
                            let actor = crate::peer::actor::PeerActor::new(
                                addr,
                                info_hash,
                                peer_id,
                                event_tx,
                                cmd_rx,
                            );
                            actor.run_incoming(stream, reserved_bytes, their_peer_id).await;
                        });
                    }
                }

                // Handle disk events
                Some(event) = self.disk_rx.recv() => {
                    match event {
                        crate::download::disk::DiskEvent::FilesAllocated => {
                            // Instead of immediately downloading from 0%, we trigger a full hash check
                            // This gives us Fast-Resume-like behavior across restarts!
                            if let Some(info) = &self.torrent_info {
                                self.update_state("VERIFYING");
                                let _ = self.disk_tx.try_send(crate::download::disk::DiskCommand::VerifyAll {
                                    expected_hashes: info.pieces.clone(),
                                });
                            }
                        }
                        crate::download::disk::DiskEvent::VerificationComplete => {
                            self.files_allocated = true;
                            
                            if let Some(info) = &self.torrent_info {
                                if self.downloaded_pieces.len() == info.pieces.len() {
                                    if self.initial_state == "PAUSED" {
                                        self.update_state("PAUSED");
                                        break;
                                    }
                                    self.update_state("SEEDING");
                                    if let Ok(mut st) = self.shared_state.write()
                                        && let Some(ts) = st.torrents.get_mut(&self.info_hash_hex) {
                                            ts.progress = 100.0;
                                        }
                                } else if self.initial_state == "PAUSED" {
                                    self.update_state("PAUSED");
                                    break;
                                } else if self.initial_state == "COMPLETED" || self.initial_state == "SEEDING" {
                                    // User manually deleted the files! Don't redownload without permission.
                                    self.update_state("ERROR_MISSING_FILES");
                                    break;
                                } else {
                                    self.update_state("DOWNLOADING");
                                }
                            }
                            
                            // Express interest to all peers so they unchoke us
                            for tx in self.peer_channels.values() {
                                let _ = tx.try_send(PeerCommand::SendInterested);
                            }
                            self.try_request_blocks();
                        }
                        crate::download::disk::DiskEvent::HashValid { piece_index } => {
                            // println!("[Manager] ✅ Piece {} successfully downloaded and verified!", piece_index);
                            self.downloaded_pieces.insert(piece_index);
                            self.blocks_received.remove(&piece_index);
                            self.try_request_blocks();

                            if let Some(info) = &self.torrent_info {
                                let progress = (self.downloaded_pieces.len() as f64 / info.pieces.len() as f64) * 100.0;
                                if let Ok(mut st) = self.shared_state.write()
                                    && let Some(ts) = st.torrents.get_mut(&self.info_hash_hex) {
                                        ts.progress = progress;
                                        ts.piece_map.push(piece_index);
                                    }

                                if self.downloaded_pieces.len() == info.pieces.len() {
                                    // println!("[Manager] 🚀 TORRENT DOWNLOAD COMPLETE!");
                                    self.update_state("SEEDING");
                                    if let Ok(mut st) = self.shared_state.write()
                                        && let Some(ts) = st.torrents.get_mut(&self.info_hash_hex) {
                                            ts.progress = 100.0;
                                        }
                                }
                            }
                        }
                        crate::download::disk::DiskEvent::HashInvalid { piece_index } => {
                            // println!("[Manager] ❌ Piece {} failed hash check!", piece_index);
                            self.blocks_received.remove(&piece_index);
                            self.requested_blocks.retain(|&(idx, _), _| idx != piece_index);
                            self.try_request_blocks();
                        }
                        crate::download::disk::DiskEvent::Error(_err) => {
                            // println!("[Manager] ❌ Disk Error: {}", err);
                        }
                        crate::download::disk::DiskEvent::BlockRead { peer, piece_index, offset, data } => {
                            if let Some(tx) = self.peer_channels.get(&peer) {
                                let _ = tx.try_send(PeerCommand::SendPiece { index: piece_index, offset, data });
                            }
                        }
                        _ => {}
                    }
                }

                // 2. Intake parsed events from active PeerActors
                Some((peer_addr, event)) = self.event_rx.recv() => {
                    match event {
                        PeerEvent::HandshakeSuccess => {
                            // Do nothing immediately. Wait for ExtensionHandshake if we want metadata.
                        }
                        PeerEvent::ExtensionHandshake { ut_metadata_id: _, metadata_size } => {
                            if self.metadata_complete { continue; }

                            if self.metadata_size.is_none() {
                                // println!("[Manager] Magnet resolution started. Metadata size: {}", metadata_size);
                                self.metadata_size = Some(metadata_size);
                                self.metadata_buffer = vec![0u8; metadata_size as usize];
                                if let Ok(mut st) = self.shared_state.write()
                                    && let Some(ts) = st.torrents.get_mut(&self.info_hash_hex) {
                                        ts.state_string = "FETCHING_METADATA".to_string();
                                    }
                            }

                            let total_pieces = metadata_size.div_ceil(16384);
                            for i in 0..total_pieces {
                                if !self.metadata_pieces_requested.contains(&i) && !self.metadata_pieces_received.contains(&i) {
                                    self.metadata_pieces_requested.insert(i);
                                    if let Some(tx) = self.peer_channels.get(&peer_addr) {
                                        let _ = tx.send(PeerCommand::RequestMetadata { piece: i }).await;
                                    }
                                    break;
                                }
                            }
                        }
                        PeerEvent::Metadata(piece_index, data) => {
                            if self.metadata_complete { continue; }
                            // println!("[Manager] Received Metadata Piece {} (Size: {}) from {}", piece_index, data.len(), peer_addr);

                            if let Some(total_size) = self.metadata_size
                                && !self.metadata_pieces_received.contains(&piece_index) {
                                    self.metadata_pieces_received.insert(piece_index);

                                    let offset = (piece_index * 16384) as usize;
                                    let end = (offset + data.len()).min(total_size as usize);
                                    let len_to_copy = end - offset;
                                    self.metadata_buffer[offset..end].copy_from_slice(&data[..len_to_copy]);

                                    let total_pieces = total_size.div_ceil(16384);
                                    if self.metadata_pieces_received.len() as u32 == total_pieces {
                                        // println!("[Manager] All metadata pieces received! Verifying SHA-1...");

                                        let hash = sha1_smol::Sha1::from(&self.metadata_buffer).digest().bytes();
                                        if hash == self.info_hash {
                                            self.metadata_complete = true;
                                            // println!("[Manager] ✅ MAGNET RESOLVED SUCCESSFULLY! Hash matched.");

                                            // Parse the canonical TorrentInfo model
                                            match crate::metadata::TorrentInfo::from_bytes(&self.metadata_buffer) {
                                                Ok(torrent_info) => {
                                                    // println!("[Manager] 🎯 TORRENT NAME: {}", torrent_info.name);
                                                    // println!("[Manager] 📦 TOTAL SIZE: {} bytes", torrent_info.total_size);
                                                    // println!("[Manager] 🧩 PIECE LENGTH: {} bytes", torrent_info.piece_length);
                                                    // println!("[Manager] 📂 FILES: {}", torrent_info.files.len());

                                                    self.torrent_info = Some(torrent_info.clone());

                                                    if let Ok(mut st) = self.shared_state.write()
                                                        && let Some(ts) = st.torrents.get_mut(&self.info_hash_hex) {
                                                            ts.torrent_name = torrent_info.name.clone();
                                                            ts.state_string = "ALLOCATING".to_string();
                                                            ts.total_pieces = torrent_info.pieces.len() as u32;
                                                            ts.total_bytes = torrent_info.total_size as f64;
                                                        }

                                                    // Transition to Disk Allocation
                                                    let _ = self.disk_tx.send(crate::download::disk::DiskCommand::AllocateFiles {
                                                        torrent_info
                                                    }).await;
                                                }
                                                Err(_e) => {
                                                    // println!("[Manager] ❌ CRITICAL: Failed to parse TorrentInfo: {}", e);
                                                }
                                            }
                                        } else {
                                            // println!("[Manager] ❌ CRITICAL: Metadata hash mismatch. Dropping metadata.");
                                            self.metadata_pieces_received.clear();
                                            self.metadata_pieces_requested.clear();
                                        }
                                    } else {
                                        // Request the next missing piece from this peer
                                        for i in 0..total_pieces {
                                            if !self.metadata_pieces_requested.contains(&i) && !self.metadata_pieces_received.contains(&i) {
                                                self.metadata_pieces_requested.insert(i);
                                                if let Some(tx) = self.peer_channels.get(&peer_addr) {
                                                    let _ = tx.send(PeerCommand::RequestMetadata { piece: i }).await;
                                                }
                                                break;
                                            }
                                        }
                                    }
                                }
                        }
                        PeerEvent::Choked => {
                            self.unchoked_peers.remove(&peer_addr);
                            self.update_peers_snapshot();
                        }
                        PeerEvent::Unchoked => {
                            self.unchoked_peers.insert(peer_addr);
                            self.try_request_blocks();
                            self.update_peers_snapshot();
                        }
                        PeerEvent::Interested => {
                            self.peers_interested_in_us.insert(peer_addr);
                        }
                        PeerEvent::NotInterested => {
                            self.peers_interested_in_us.remove(&peer_addr);
                        }
                        PeerEvent::Request { index, offset, length } => {
                            if self.peers_we_unchoked.contains(&peer_addr) {
                                // We are unchoking them, fulfill request!
                                let _ = self.disk_tx.try_send(crate::download::disk::DiskCommand::ReadBlock {
                                    piece_index: index,
                                    offset,
                                    length,
                                    peer: peer_addr,
                                });
                            }
                        }
                        PeerEvent::Have(index) => {
                            // 1. Primary write to Swarm Graph
                            self.swarm_state_mut.peer_to_pieces.entry(peer_addr).or_default().insert(index);
                            let is_new = self.swarm_state_mut.piece_to_peers.entry(index).or_default().insert(peer_addr);
                            if is_new {
                                *self.swarm_state_mut.piece_rarity.entry(index).or_default() += 1;
                            }

                            // 2. Legacy compatibility shim
                            let entry = self.peer_bitfields.entry(peer_addr).or_default();
                            entry.insert(index);

                            if !self.downloaded_pieces.contains(&index)
                                && let Some(tx) = self.peer_channels.get(&peer_addr) {
                                    let _ = tx.try_send(PeerCommand::SendInterested);
                                }

                            self.maybe_update_rarest_pieces();
                            self.try_request_blocks();
                        }
                        PeerEvent::Bitfield(bitfield) => {
                            let mut set = HashSet::new();
                            let mut interested = false;
                            for (byte_idx, &byte) in bitfield.iter().enumerate() {
                                for bit_idx in 0..8 {
                                    if (byte & (1 << (7 - bit_idx))) != 0 {
                                        let index = (byte_idx * 8 + bit_idx) as u32;
                                        set.insert(index);

                                        // 1. Primary write to Swarm Graph
                                        let is_new = self.swarm_state_mut.piece_to_peers.entry(index).or_default().insert(peer_addr);
                                        if is_new {
                                            *self.swarm_state_mut.piece_rarity.entry(index).or_default() += 1;
                                        }

                                        if !self.downloaded_pieces.contains(&index) {
                                            interested = true;
                                        }
                                    }
                                }
                            }
                            self.swarm_state_mut.peers.insert(peer_addr);
                            self.swarm_state_mut.peer_to_pieces.insert(peer_addr, set.clone());

                            // 2. Legacy compatibility shim
                            self.peer_bitfields.insert(peer_addr, set);

                            if interested
                                && let Some(tx) = self.peer_channels.get(&peer_addr) {
                                    let _ = tx.try_send(PeerCommand::SendInterested);
                                }

                            self.maybe_update_rarest_pieces();
                            self.try_request_blocks();
                        }
                        PeerEvent::Piece { index, offset, data } => {
                            if let Some(in_flight) = self.peer_in_flight.get_mut(&peer_addr)
                                && *in_flight > 0 { *in_flight -= 1; }
                            let data_len = data.len();
                            let mut latency_ms = None;
                            if let Some(reqs) = self.requested_blocks.get_mut(&(index, offset))
                                && let Some(idx) = reqs.iter().position(|r| r.0 == peer_addr) {
                                    let (_, req_time) = reqs.remove(idx);
                                    latency_ms = Some(req_time.elapsed().as_millis() as f64);
                                }

                            let tel = self.peer_telemetry.entry(peer_addr).or_default();
                            tel.bytes_downloaded += data_len as u64;
                            // Deduct from speed limit token bucket
                            let limit_val = self.download_speed_limit.load(std::sync::atomic::Ordering::Relaxed);
                            if limit_val > 0 {
                                self.token_bucket_bytes -= data_len as f64;
                            }
                            if let Some(lat) = latency_ms {
                                if tel.latency_samples == 0 {
                                    tel.latency_ema_ms = lat;
                                } else {
                                    tel.latency_ema_ms = (lat * 0.2) + (tel.latency_ema_ms * 0.8);
                                }
                                tel.latency_samples += 1;
                            }

                            // NOTE: update_peers_snapshot moved to the 2s tick to avoid per-block overhead
                            if let Ok(mut st) = self.shared_state.write()
                                && let Some(ts) = st.torrents.get_mut(&self.info_hash_hex) {
                                    ts.total_downloaded += data_len as f64;
                                }

                            if let Some(info) = &self.torrent_info {
                                let mut piece_size = info.piece_length;
                                if index == info.pieces.len() as u32 - 1 {
                                    let rem = (info.total_size % info.piece_length as u64) as u32;
                                    if rem > 0 { piece_size = rem; }
                                }

                                let piece_buf = self.active_piece_buffers.entry(index).or_insert_with(|| vec![0u8; piece_size as usize]);

                                let end = (offset as usize + data.len()).min(piece_size as usize);
                                if offset as usize <= piece_buf.len() && end <= piece_buf.len() {
                                    piece_buf[offset as usize .. end].copy_from_slice(&data[.. (end - offset as usize)]);
                                }

                                let blocks = self.blocks_received.entry(index).or_default();
                                blocks.insert(offset);

                                let total_blocks = piece_size.div_ceil(16384);
                                if blocks.len() as u32 == total_blocks {
                                    let expected_hash = info.pieces[index as usize];
                                    let hash = tokio::task::block_in_place(|| {
                                        let mut hasher = sha1_smol::Sha1::new();
                                        hasher.update(piece_buf);
                                        hasher.digest().bytes()
                                    });

                                    if hash == expected_hash {
                                        let full_data = self.active_piece_buffers.remove(&index).unwrap();
                                        // Send directly to Disk Actor as one complete piece
                                        let _ = self.disk_tx.send(crate::download::disk::DiskCommand::WritePiece {
                                            piece_index: index,
                                            offset: 0,
                                            data: full_data,
                                        }).await;

                                        // Update state immediately without waiting for disk readback
                                        self.downloaded_pieces.insert(index);
                                        self.requested_blocks.retain(|&(idx, _), _| idx != index);
                                        self.blocks_received.remove(&index);

                                        for tx in self.peer_channels.values() {
                                            let _ = tx.try_send(crate::peer::actor::PeerCommand::Have { piece_index: index });
                                        }

                                        // Canonical progress: verified pieces only (monotonically increasing)
                                        let progress = (self.downloaded_pieces.len() as f64 / info.pieces.len() as f64) * 100.0;
                                        if let Ok(mut st) = self.shared_state.write()
                                            && let Some(ts) = st.torrents.get_mut(&self.info_hash_hex) {
                                                ts.progress = progress;
                                                ts.piece_map.push(index);
                                            }

                                        if self.downloaded_pieces.len() == info.pieces.len()
                                            && let Ok(mut st) = self.shared_state.write()
                                                && let Some(ts) = st.torrents.get_mut(&self.info_hash_hex) {
                                                    ts.state_string = "SEEDING".to_string();
                                                    ts.progress = 100.0;
                                                }
                                    } else {
                                        // Hash mismatch! Drop the buffer and request blocks again
                                        // eprintln!("[Manager] ❌ HASH MISMATCH for piece {}! Expected: {:?}, Got: {:?}", index, expected_hash, hash);
                                        self.active_piece_buffers.remove(&index);
                                        self.blocks_received.remove(&index);
                                        self.requested_blocks.retain(|&(idx, _), _| idx != index);
                                        // No progress rollback needed — progress is based solely on verified pieces
                                    }
                                }
                            }

                            // Only re-request when a peer has capacity freed up (piece completed above)
                            // The 2s tick also calls try_request_blocks as a fallback
                            self.try_request_blocks();
                        }
                        PeerEvent::PexPeers(peers) => {
                            for peer in peers {
                                self.try_spawn_peer(peer);
                            }
                        }
                        PeerEvent::Disconnected => {
                            // 1. Primary write to Swarm Graph
                            if let Some(pieces) = self.swarm_state_mut.peer_to_pieces.remove(&peer_addr) {
                                for piece in pieces {
                                    if let Some(set) = self.swarm_state_mut.piece_to_peers.get_mut(&piece) {
                                        set.remove(&peer_addr);
                                        if let Some(r) = self.swarm_state_mut.piece_rarity.get_mut(&piece) {
                                            *r = r.saturating_sub(1);
                                        }
                                    }
                                }
                            }
                            self.swarm_state_mut.peers.remove(&peer_addr);

                            // 2. Legacy shim
                            self.peer_channels.remove(&peer_addr);
                            self.unchoked_peers.remove(&peer_addr);
                            self.peer_bitfields.remove(&peer_addr);
                            self.peer_in_flight.remove(&peer_addr);
                            self.peer_telemetry.remove(&peer_addr);
                            for requesters in self.requested_blocks.values_mut() {
                                requesters.retain(|(addr, _)| *addr != peer_addr);
                            }

                            self.maybe_update_rarest_pieces();

                            // Pool slot opened up! Immediately replenish from the queue.
                            if let Some(next_ip) = self.queued_ips.pop_front() {
                                self.try_spawn_peer(next_ip);
                            }
                            self.update_peers_snapshot();
                        }
                        // All variants of PeerEvent are explicitly matched.
                    }
                }
            }
        }
    }
}

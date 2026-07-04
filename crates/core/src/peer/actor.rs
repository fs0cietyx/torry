use std::net::SocketAddr;
use tokio::sync::mpsc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use super::mse::PeerStream;

/// Messages sent FROM the isolated Peer Actor TO the central Torrent Manager.
fn find_bencode_dict_end(buf: &[u8]) -> Option<usize> {
    if buf.is_empty() || buf[0] != b'd' {
        return None;
    }
    let mut i = 1;
    let mut depth = 1;
    while i < buf.len() && depth > 0 {
        match buf[i] {
            b'd' | b'l' => {
                depth += 1;
                i += 1;
            }
            b'e' => {
                depth -= 1;
                i += 1;
            }
            b'i' => {
                i += 1;
                while i < buf.len() && buf[i] != b'e' {
                    i += 1;
                }
                i += 1; // skip 'e'
            }
            b'0'..=b'9' => {
                let mut len = 0;
                while i < buf.len() && buf[i] >= b'0' && buf[i] <= b'9' {
                    len = len * 10 + (buf[i] - b'0') as usize;
                    i += 1;
                }
                if i < buf.len() && buf[i] == b':' {
                    i += 1;
                    i += len;
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }
    if depth == 0 { Some(i) } else { None }
}

#[derive(Debug)]
pub enum PeerEvent {
    HandshakeSuccess,
    Disconnected,
    Choked,
    Unchoked,
    Have(u32),
    Bitfield(Vec<u8>),
    Piece {
        index: u32,
        offset: u32,
        data: Vec<u8>,
    },
    Metadata(u32, Vec<u8>), // Metadata piece for BEP 9
    ExtensionHandshake {
        ut_metadata_id: u8,
        metadata_size: u32,
    },
    PexPeers(Vec<SocketAddr>),
    Interested,
    NotInterested,
    Request {
        index: u32,
        offset: u32,
        length: u32,
    },
}

/// Commands sent FROM the central Torrent Manager TO the isolated Peer Actor.
#[derive(Debug)]
pub enum PeerCommand {
    SendInterested,
    RequestPiece {
        index: u32,
        offset: u32,
        length: u32,
    },
    RequestMetadata {
        piece: u32,
    },
    Have {
        piece_index: u32,
    },
    Choke,
    Unchoke,
    SendPex {
        peers: Vec<SocketAddr>,
    },
    SendPiece {
        index: u32,
        offset: u32,
        data: Vec<u8>,
    },
    Disconnect,
}

pub struct PeerActor {
    addr: SocketAddr,
    info_hash: [u8; 20],
    peer_id: [u8; 20],
    manager_tx: mpsc::Sender<(SocketAddr, PeerEvent)>,
    cmd_rx: mpsc::Receiver<PeerCommand>,
}

impl PeerActor {
    pub fn new(
        addr: SocketAddr,
        info_hash: [u8; 20],
        peer_id: [u8; 20],
        manager_tx: mpsc::Sender<(SocketAddr, PeerEvent)>,
        cmd_rx: mpsc::Receiver<PeerCommand>,
    ) -> Self {
        Self {
            addr,
            info_hash,
            peer_id,
            manager_tx,
            cmd_rx,
        }
    }

    /// The isolated Tokio task loop for this specific peer connection.
    pub async fn run(self) {
        //"[Actor] run started for {:?}", self.addr);
        // 1. Establish connection (races TCP and uTP) with a timeout
        let stream = match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            crate::peer::stream::PeerStream::connect(self.addr),
        )
        .await
        {
            Ok(Ok(s)) => {
                let _ = s.set_buffer_sizes(4 * 1024 * 1024, 4 * 1024 * 1024);
                let _ = s.set_nodelay(true);
                let _ = s.set_keepalive(true);
                s
            }
            Ok(Err(_e)) => {
                //"[Actor] connection failed for {:?}: {}", self.addr, e);
                let _ = self
                    .manager_tx
                    .send((self.addr, PeerEvent::Disconnected))
                    .await;
                return;
            }
            Err(_) => {
                //"[Actor] connection timeout for {:?}", self.addr);
                let _ = self
                    .manager_tx
                    .send((self.addr, PeerEvent::Disconnected))
                    .await;
                return;
            }
        };

        // Disable Nagle's algorithm and set TCP keepalive
        let _ = stream.set_nodelay(true);
        let _ = stream.set_keepalive(true);

        let mut stream = match tokio::time::timeout(
            std::time::Duration::from_millis(2500),
            super::mse::mse_handshake(stream, &self.info_hash),
        )
        .await
        {
            Ok(Ok(encrypted_stream)) => {
                encrypted_stream
            }
            Ok(Err((_, _err))) => {
                let stream2 = match tokio::time::timeout(
                    std::time::Duration::from_secs(3),
                    crate::peer::stream::PeerStream::connect(self.addr),
                ).await {
                    Ok(Ok(s)) => s,
                    _ => {
                        let _ = self.manager_tx.send((self.addr, PeerEvent::Disconnected)).await;
                        return;
                    }
                };
                let _ = stream2.set_nodelay(true);
                let _ = stream2.set_keepalive(true);
                PeerStream::plaintext(stream2)
            }
            Err(_) => {
                let stream2 = match tokio::time::timeout(
                    std::time::Duration::from_secs(3),
                    crate::peer::stream::PeerStream::connect(self.addr),
                ).await {
                    Ok(Ok(s)) => s,
                    _ => {
                        let _ = self.manager_tx.send((self.addr, PeerEvent::Disconnected)).await;
                        return;
                    }
                };
                let _ = stream2.set_nodelay(true);
                let _ = stream2.set_keepalive(true);
                PeerStream::plaintext(stream2)
            }
        };

        // 2. Perform TCP Handshake (BEP 3 & BEP 10)
        let mut handshake = vec![19u8]; // Protocol length
        handshake.extend_from_slice(b"BitTorrent protocol");

        // Reserved bytes: enable Extension Protocol (BEP 10)
        let mut reserved = [0u8; 8];
        reserved[5] |= 0x10; // Bit 43 from right is byte 5, bit 4
        handshake.extend_from_slice(&reserved);

        handshake.extend_from_slice(&self.info_hash);
        handshake.extend_from_slice(&self.peer_id);

        if stream.write_all(&handshake).await.is_err() {
            let _ = self
                .manager_tx
                .send((self.addr, PeerEvent::Disconnected))
                .await;
            return;
        }

        // Read handshake response (68 bytes)
        let mut response = [0u8; 68];
        match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            stream.read_exact(&mut response),
        )
        .await
        {
            Ok(Ok(_)) => {
                //"[Actor] TCP handshake received from {:?}", self.addr);
            }
            Ok(Err(_e)) => {
                //"[Actor] TCP handshake error from {:?}: {}", self.addr, e);
                let _ = self
                    .manager_tx
                    .send((self.addr, PeerEvent::Disconnected))
                    .await;
                return;
            }
            Err(_) => {
                //"[Actor] TCP handshake timeout from {:?}", self.addr);
                let _ = self
                    .manager_tx
                    .send((self.addr, PeerEvent::Disconnected))
                    .await;
                return;
            }
        }

        // Validate protocol string and info_hash
        if response[0] != 19
            || &response[1..20] != b"BitTorrent protocol"
            || response[28..48] != self.info_hash
        {
            //"[Actor] Invalid TCP handshake response from {:?}", self.addr);
            let _ = self
                .manager_tx
                .send((self.addr, PeerEvent::Disconnected))
                .await;
            return;
        }

        let supports_extensions = (response[25] & 0x10) != 0;

        // 3. Perform Extension Protocol Handshake (BEP 10)
        if supports_extensions {
            #[derive(serde::Serialize)]
            struct ExtHandshake {
                m: std::collections::HashMap<String, u8>,
            }
            let mut m = std::collections::HashMap::new();
            m.insert("ut_metadata".to_string(), 1);
            m.insert("ut_pex".to_string(), 2);

            let ext_msg = ExtHandshake { m };
            let bencoded = serde_bencode::to_bytes(&ext_msg).unwrap_or_default();

            let len = (2 + bencoded.len()) as u32;

            let mut ext_frame = Vec::new();
            ext_frame.extend_from_slice(&len.to_be_bytes());
            ext_frame.push(20); // Extension Message
            ext_frame.push(0); // Handshake ID
            ext_frame.extend_from_slice(&bencoded);

            if stream.write_all(&ext_frame).await.is_err() {
                let _ = self
                    .manager_tx
                    .send((self.addr, PeerEvent::Disconnected))
                    .await;
                return;
            }
        }

        let _ = self
            .manager_tx
            .send((self.addr, PeerEvent::HandshakeSuccess))
            .await;

        self.connection_loop(stream).await;
    }

    pub async fn run_incoming(self, raw_stream: crate::peer::stream::PeerStream, reserved: [u8; 8], _their_peer_id: [u8; 20]) {
        let _ = raw_stream.set_buffer_sizes(4 * 1024 * 1024, 4 * 1024 * 1024);
        let _ = raw_stream.set_nodelay(true);
        
        let mut stream = PeerStream::plaintext(raw_stream);

        // 1. Send OUR handshake
        let mut handshake = vec![19u8]; // Protocol length
        handshake.extend_from_slice(b"BitTorrent protocol");
        let mut our_reserved = [0u8; 8];
        our_reserved[5] |= 0x10; // Enable extensions
        handshake.extend_from_slice(&our_reserved);
        handshake.extend_from_slice(&self.info_hash);
        handshake.extend_from_slice(&self.peer_id);

        if stream.write_all(&handshake).await.is_err() {
            let _ = self.manager_tx.send((self.addr, PeerEvent::Disconnected)).await;
            return;
        }

        let supports_extensions = (reserved[5] & 0x10) != 0;

        if supports_extensions {
            #[derive(serde::Serialize)]
            struct ExtHandshake {
                m: std::collections::HashMap<String, u8>,
            }
            let mut m = std::collections::HashMap::new();
            m.insert("ut_metadata".to_string(), 1);
            m.insert("ut_pex".to_string(), 2);

            let ext_msg = ExtHandshake { m };
            let bencoded = serde_bencode::to_bytes(&ext_msg).unwrap_or_default();
            let len = (2 + bencoded.len()) as u32;

            let mut ext_frame = Vec::new();
            ext_frame.extend_from_slice(&len.to_be_bytes());
            ext_frame.push(20);
            ext_frame.push(0);
            ext_frame.extend_from_slice(&bencoded);

            if stream.write_all(&ext_frame).await.is_err() {
                let _ = self.manager_tx.send((self.addr, PeerEvent::Disconnected)).await;
                return;
            }
        }

        let _ = self.manager_tx.send((self.addr, PeerEvent::HandshakeSuccess)).await;

        self.connection_loop(stream).await;
    }

    async fn connection_loop(mut self, mut stream: PeerStream<crate::peer::stream::PeerStream>) {
        let mut peer_ut_metadata_id = None;
        let mut len_bytes = [0u8; 4];
        let mut msg_buf = Vec::new();
        loop {
            tokio::select! {
                cmd = self.cmd_rx.recv() => {
                    match cmd {
                        Some(PeerCommand::RequestMetadata { piece }) => {
                            if let Some(id) = peer_ut_metadata_id {
                                #[derive(serde::Serialize)]
                                struct MetadataReq {
                                    msg_type: u8,
                                    piece: u32,
                                }
                                let req = MetadataReq { msg_type: 0, piece };
                                let bencoded = serde_bencode::to_bytes(&req).unwrap_or_default();

                                let len = (2 + bencoded.len()) as u32;
                                let mut ext_frame = Vec::new();
                                ext_frame.extend_from_slice(&len.to_be_bytes());
                                ext_frame.push(20);
                                ext_frame.push(id);
                                ext_frame.extend_from_slice(&bencoded);

                                if stream.write_all(&ext_frame).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Some(PeerCommand::SendInterested) => {
                            let msg = [0, 0, 0, 1, 2];
                            if stream.write_all(&msg).await.is_err() { break; }
                        }
                        Some(PeerCommand::RequestPiece { index, offset, length }) => {
                            let mut msg = Vec::with_capacity(17);
                            msg.extend_from_slice(&13u32.to_be_bytes());
                            msg.push(6);
                            msg.extend_from_slice(&index.to_be_bytes());
                            msg.extend_from_slice(&offset.to_be_bytes());
                            msg.extend_from_slice(&length.to_be_bytes());

                            if stream.write_all(&msg).await.is_err() { break; }
                        }
                        Some(PeerCommand::Have { piece_index }) => {
                            let mut msg = Vec::with_capacity(9);
                            msg.extend_from_slice(&5u32.to_be_bytes());
                            msg.push(4);
                            msg.extend_from_slice(&piece_index.to_be_bytes());
                            if stream.write_all(&msg).await.is_err() { break; }
                        }
                        Some(PeerCommand::Choke) => {
                            let msg = [0, 0, 0, 1, 0];
                            if stream.write_all(&msg).await.is_err() { break; }
                        }
                        Some(PeerCommand::Unchoke) => {
                            let msg = [0, 0, 0, 1, 1];
                            if stream.write_all(&msg).await.is_err() { break; }
                        }
                        Some(PeerCommand::SendPiece { index, offset, data }) => {
                            let len = 9 + data.len() as u32;
                            let mut msg = Vec::with_capacity(4 + len as usize);
                            msg.extend_from_slice(&len.to_be_bytes());
                            msg.push(7);
                            msg.extend_from_slice(&index.to_be_bytes());
                            msg.extend_from_slice(&offset.to_be_bytes());
                            msg.extend_from_slice(&data);
                            if stream.write_all(&msg).await.is_err() { break; }
                        }
                        Some(PeerCommand::SendPex { peers }) => {
                            let mut ipv4_peers = Vec::new();
                            for addr in peers {
                                if let std::net::SocketAddr::V4(v4) = addr {
                                    ipv4_peers.extend_from_slice(&v4.ip().octets());
                                    ipv4_peers.extend_from_slice(&v4.port().to_be_bytes());
                                }
                            }
                            
                            #[derive(serde::Serialize)]
                            struct PexMsg {
                                added: serde_bytes::ByteBuf,
                            }
                            
                            let msg = PexMsg {
                                added: serde_bytes::ByteBuf::from(ipv4_peers),
                            };
                            
                            if let Ok(bencoded) = serde_bencode::to_bytes(&msg) {
                                let len = (2 + bencoded.len()) as u32;
                                let mut ext_frame = Vec::new();
                                ext_frame.extend_from_slice(&len.to_be_bytes());
                                ext_frame.push(20);
                                ext_frame.push(1); // Placeholder for pex id
                                ext_frame.extend_from_slice(&bencoded);
                                
                                let _ = stream.write_all(&ext_frame).await;
                            }
                        }
                        Some(PeerCommand::Disconnect) | None => break,
                    }
                }
                res = stream.read_exact(&mut len_bytes) => {
                    match res {
                        Ok(_) => {
                            //"[Actor] Read len bytes: {:?}", len_bytes);
                            let msg_len = u32::from_be_bytes(len_bytes);
                            if msg_len == 0 { continue; }
                            if msg_len > 2_000_000 { 
                                //"[Actor] msg_len > 2_000_000: {}", msg_len);
                                break; 
                            }

                            msg_buf.resize(msg_len as usize, 0);
                            if let Err(_e) = stream.read_exact(&mut msg_buf).await {
                                //"[Actor] read_exact payload failed: {}", e);
                                break;
                            }

                            //"[Actor] Received msg id: {}", msg_buf[0]);
                            let msg_id = msg_buf[0];
                            match msg_id {
                                0 => { let _ = self.manager_tx.send((self.addr, PeerEvent::Choked)).await; }
                                1 => { let _ = self.manager_tx.send((self.addr, PeerEvent::Unchoked)).await; }
                                2 => { let _ = self.manager_tx.send((self.addr, PeerEvent::Interested)).await; }
                                3 => { let _ = self.manager_tx.send((self.addr, PeerEvent::NotInterested)).await; }
                                4 => {
                                    if msg_len == 5 {
                                        let mut piece_index_bytes = [0u8; 4];
                                        piece_index_bytes.copy_from_slice(&msg_buf[1..5]);
                                        let index = u32::from_be_bytes(piece_index_bytes);
                                        let _ = self.manager_tx.send((self.addr, PeerEvent::Have(index))).await;
                                    }
                                }
                                5 => {
                                    let bitfield = msg_buf[1..].to_vec();
                                    let _ = self.manager_tx.send((self.addr, PeerEvent::Bitfield(bitfield))).await;
                                }
                                6 => {
                                    if msg_len == 13 {
                                        let mut index_bytes = [0u8; 4];
                                        index_bytes.copy_from_slice(&msg_buf[1..5]);
                                        let index = u32::from_be_bytes(index_bytes);

                                        let mut offset_bytes = [0u8; 4];
                                        offset_bytes.copy_from_slice(&msg_buf[5..9]);
                                        let offset = u32::from_be_bytes(offset_bytes);

                                        let mut length_bytes = [0u8; 4];
                                        length_bytes.copy_from_slice(&msg_buf[9..13]);
                                        let length = u32::from_be_bytes(length_bytes);

                                        let _ = self.manager_tx.send((self.addr, PeerEvent::Request { index, offset, length })).await;
                                    }
                                }
                                7 => {
                                    if msg_len > 9 {
                                        let mut index_bytes = [0u8; 4];
                                        index_bytes.copy_from_slice(&msg_buf[1..5]);
                                        let index = u32::from_be_bytes(index_bytes);

                                        let mut offset_bytes = [0u8; 4];
                                        offset_bytes.copy_from_slice(&msg_buf[5..9]);
                                        let offset = u32::from_be_bytes(offset_bytes);

                                        let data = msg_buf[9..].to_vec();
                                        let _ = self.manager_tx.send((self.addr, PeerEvent::Piece { index, offset, data })).await;
                                    }
                                }
                                20 if msg_len > 1 => {
                                    let ext_id = msg_buf[1];
                                    if ext_id == 0 {
                                        use serde::Deserialize;
                                        #[derive(Deserialize, Debug)]
                                        struct ExtHandshakeResp {
                                            m: Option<std::collections::HashMap<String, u8>>,
                                            metadata_size: Option<u32>,
                                        }
                                        match serde_bencode::from_bytes::<ExtHandshakeResp>(&msg_buf[2..]) {
                                            Ok(resp) => {
                                                if let (Some(m), Some(metadata_size)) = (resp.m.as_ref(), resp.metadata_size) {
                                                    if let Some(&ut_metadata_id) = m.get("ut_metadata") {
                                                        peer_ut_metadata_id = Some(ut_metadata_id);
                                                        let _ = self.manager_tx.send((self.addr, PeerEvent::ExtensionHandshake {
                                                            ut_metadata_id,
                                                            metadata_size,
                                                        })).await;
                                                    } else {
                                                        //"[Actor] Handshake parsed, but ut_metadata missing in m: {:?}", m);
                                                    }
                                                } else {
                                                    //"[Actor] Handshake parsed, but missing m or metadata_size: {:?}", resp);
                                                }
                                            }
                                            Err(_e) => {
                                                //"[Actor] Failed to parse ExtHandshakeResp: {}", e);
                                            }
                                        }
                                    } else if ext_id == 1 {
                                        if let Some(dict_end) = find_bencode_dict_end(&msg_buf[2..]) {
                                            let dict_bytes = &msg_buf[2..2+dict_end];
                                            use serde::Deserialize;
                                            #[derive(Deserialize, Debug)]
                                            struct MetadataMsg {
                                                msg_type: u8,
                                                piece: u32,
                                            }
                                            if let Ok(msg) = serde_bencode::from_bytes::<MetadataMsg>(dict_bytes)
                                                && msg.msg_type == 1 {
                                                    let raw_data = msg_buf[2+dict_end..].to_vec();
                                                    let _ = self.manager_tx.send((self.addr, PeerEvent::Metadata(msg.piece, raw_data))).await;
                                                }
                                        }
                                    } else if ext_id == 2 {
                                        use serde::Deserialize;
                                        #[derive(Deserialize, Debug)]
                                        struct PexMsg {
                                            added: Option<serde_bytes::ByteBuf>,
                                        }
                                        if let Ok(msg) = serde_bencode::from_bytes::<PexMsg>(&msg_buf[2..])
                                            && let Some(added) = msg.added {
                                                let mut peers = Vec::new();
                                                for chunk in added.chunks_exact(6) {
                                                    let ip = std::net::Ipv4Addr::new(chunk[0], chunk[1], chunk[2], chunk[3]);
                                                    let port = u16::from_be_bytes([chunk[4], chunk[5]]);
                                                    peers.push(SocketAddr::V4(std::net::SocketAddrV4::new(ip, port)));
                                                }
                                                if !peers.is_empty() {
                                                    let _ = self.manager_tx.send((self.addr, PeerEvent::PexPeers(peers))).await;
                                                }
                                            }
                                    }
                                }
                                _ => {}
                            }
                        }
                        Err(_e) => {
                            //"[Actor] read_exact len_bytes failed: {}", e);
                            break;
                        }
                    }
                }
            }
        }

        let _ = self.manager_tx.send((self.addr, PeerEvent::Disconnected)).await;
    }
}

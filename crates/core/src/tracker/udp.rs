use bytes::{Buf, BufMut, BytesMut};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::UdpSocket;

use super::{AnnounceRequest, AnnounceResponse, TrackerEvent};

const UDP_PROTOCOL_ID: u64 = 0x41727101980; // BitTorrent Magic Constant

/// A high-performance UDP Tracker client implementing BEP 15.
pub struct UdpTrackerClient {
    tracker_url: String, // e.g. "tracker.opentrackr.org:1337"
}

impl UdpTrackerClient {
    pub fn new(url: String) -> Self {
        Self { tracker_url: url }
    }

    /// The full two-step UDP sequence.
    pub async fn announce(&self, req: &AnnounceRequest) -> Result<AnnounceResponse, String> {
        // 1. Bind to an ephemeral UDP port
        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|e| e.to_string())?;
        socket
            .connect(&self.tracker_url)
            .await
            .map_err(|e| e.to_string())?;

        // 2. Perform the Connection Handshake
        let connection_id = self.connect(&socket).await?;

        // 3. Perform the actual Announce to get peers
        self.send_announce(&socket, connection_id, req).await
    }

    async fn connect(&self, socket: &UdpSocket) -> Result<u64, String> {
        let transaction_id: u32 = rand::random();

        let mut buf = BytesMut::with_capacity(16);

        // Write Connect Request
        buf.put_u64(UDP_PROTOCOL_ID);
        buf.put_u32(0); // Action: Connect
        buf.put_u32(transaction_id);

        let mut retries = 0;
        loop {
            socket.send(&buf).await.map_err(|e| e.to_string())?;

            let mut recv_buf = [0u8; 16];
            match tokio::time::timeout(Duration::from_secs(3), socket.recv(&mut recv_buf)).await {
                Ok(Ok(_bytes_read)) => {
                    let mut response = &recv_buf[..];
                    let action = response.get_u32();
                    let rx_transaction_id = response.get_u32();

                    if action != 0 || rx_transaction_id != transaction_id {
                        return Err("Invalid connection response".into());
                    }

                    return Ok(response.get_u64());
                }
                Ok(Err(e)) => return Err(e.to_string()),
                Err(_) => {
                    retries += 1;
                    if retries >= 3 {
                        return Err("Tracker connect timeout".into());
                    }
                }
            }
        }
    }

    async fn send_announce(
        &self,
        socket: &UdpSocket,
        connection_id: u64,
        req: &AnnounceRequest,
    ) -> Result<AnnounceResponse, String> {
        let transaction_id: u32 = rand::random();

        let mut buf = BytesMut::with_capacity(98);

        buf.put_u64(connection_id);
        buf.put_u32(1); // Action: Announce
        buf.put_u32(transaction_id);
        buf.put_slice(&req.info_hash);
        buf.put_slice(&req.peer_id);
        buf.put_u64(req.downloaded);
        buf.put_u64(req.left);
        buf.put_u64(req.uploaded);

        let event_val = match req.event {
            TrackerEvent::None => 0,
            TrackerEvent::Completed => 1,
            TrackerEvent::Started => 2,
            TrackerEvent::Stopped => 3,
        };
        buf.put_u32(event_val);
        buf.put_u32(0); // IP address: 0 (default)
        buf.put_u32(rand::random()); // Key
        buf.put_i32(-1); // Num want: -1 (default)
        buf.put_u16(req.port); // Listening Port

        let mut retries = 0;
        let bytes_read;
        let mut recv_buf = [0u8; 2048];

        loop {
            socket.send(&buf).await.map_err(|e| e.to_string())?;

            match tokio::time::timeout(Duration::from_secs(3), socket.recv(&mut recv_buf)).await {
                Ok(Ok(b)) => {
                    bytes_read = b;
                    break;
                }
                Ok(Err(e)) => return Err(e.to_string()),
                Err(_) => {
                    retries += 1;
                    if retries >= 3 {
                        return Err("Tracker announce timeout".into());
                    }
                }
            }
        }

        if bytes_read < 20 {
            return Err("Announce response too short".into());
        }

        let mut response = &recv_buf[..bytes_read];
        let action = response.get_u32();
        let rx_transaction_id = response.get_u32();

        if action != 1 || rx_transaction_id != transaction_id {
            return Err("Invalid announce response".into());
        }

        let interval = response.get_u32();
        let leechers = response.get_u32();
        let seeders = response.get_u32();

        let mut peers = Vec::new();

        // Every 6 bytes after the first 20 is a peer (4 bytes IP, 2 bytes Port)
        while response.remaining() >= 6 {
            let ip_bytes = response.get_u32();
            let port = response.get_u16();

            let ip = std::net::Ipv4Addr::from(ip_bytes);
            peers.push(SocketAddr::new(std::net::IpAddr::V4(ip), port));
        }

        // Print to prove the pipeline is alive headless
        // println!("[Tracker: {}] Discovered {} peers!", self.tracker_url, peers.len());

        Ok(AnnounceResponse {
            interval,
            seeders,
            leechers,
            peers,
        })
    }
}

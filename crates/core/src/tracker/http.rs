use crate::tracker::{AnnounceRequest, AnnounceResponse, TrackerEvent};
use reqwest::Client;
use serde::Deserialize;
use serde_bytes::ByteBuf;
use std::net::{Ipv4Addr, SocketAddr};

pub struct HttpTrackerClient {
    tracker_url: String,
    client: Client,
}

#[derive(Deserialize, Debug)]
struct TrackerResponse {
    #[serde(default)]
    #[serde(rename = "failure reason")]
    failure_reason: Option<String>,
    #[serde(default)]
    interval: Option<u32>,
    #[serde(default)]
    complete: Option<u32>,
    #[serde(default)]
    incomplete: Option<u32>,
    #[serde(default)]
    peers: Option<ByteBuf>,
}

impl HttpTrackerClient {
    pub fn new(url: String) -> Self {
        Self {
            tracker_url: url,
            client: Client::new(),
        }
    }

    pub async fn announce(&self, req: &AnnounceRequest) -> Result<AnnounceResponse, String> {
        // Manually url-encode info_hash and peer_id to prevent UTF-8 corruption
        let mut info_hash_encoded = String::new();
        for b in &req.info_hash {
            info_hash_encoded.push_str(&format!("%{:02x}", b));
        }

        let mut peer_id_encoded = String::new();
        for b in &req.peer_id {
            peer_id_encoded.push_str(&format!("%{:02x}", b));
        }

        let event_str = match req.event {
            TrackerEvent::None => "",
            TrackerEvent::Completed => "completed",
            TrackerEvent::Started => "started",
            TrackerEvent::Stopped => "stopped",
        };

        // Construct base URL with query params
        let mut url = format!(
            "{}?info_hash={}&peer_id={}&port={}&uploaded={}&downloaded={}&left={}&compact=1",
            self.tracker_url,
            info_hash_encoded,
            peer_id_encoded,
            req.port,
            req.uploaded,
            req.downloaded,
            req.left
        );

        if !event_str.is_empty() {
            url.push_str(&format!("&event={}", event_str));
        }

        let response_bytes = self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .bytes()
            .await
            .map_err(|e| e.to_string())?;

        let parsed: TrackerResponse = serde_bencode::from_bytes(&response_bytes)
            .map_err(|e| format!("Failed to parse bencode: {}", e))?;

        if let Some(reason) = parsed.failure_reason {
            return Err(format!("Tracker returned error: {}", reason));
        }

        let mut peers = Vec::new();
        if let Some(peers_bytes) = parsed.peers {
            let bytes = peers_bytes.into_vec();
            if bytes.len() % 6 == 0 {
                for chunk in bytes.chunks_exact(6) {
                    let ip = Ipv4Addr::new(chunk[0], chunk[1], chunk[2], chunk[3]);
                    let port = u16::from_be_bytes([chunk[4], chunk[5]]);
                    peers.push(SocketAddr::new(std::net::IpAddr::V4(ip), port));
                }
            } else {
                return Err("Non-compact peers dictionary not supported yet".into());
            }
        }

        // println!("[Tracker: {}] Discovered {} peers!", self.tracker_url, peers.len());

        Ok(AnnounceResponse {
            interval: parsed.interval.unwrap_or(1800),
            seeders: parsed.complete.unwrap_or(0),
            leechers: parsed.incomplete.unwrap_or(0),
            peers,
        })
    }
}

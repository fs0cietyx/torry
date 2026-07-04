use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The Extended Handshake Dictionary (BEP 10).
/// This is Bencoded and sent immediately after the main 68-byte handshake.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExtendedHandshake {
    /// A mapping of extension names to our local Message IDs.
    #[serde(rename = "m")]
    pub messages: HashMap<String, u8>,

    #[serde(rename = "v", skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Only the peer who actually HAS the metadata populates this.
    /// Since we are a magnet client asking for it, ours will be None.
    #[serde(rename = "metadata_size", skip_serializing_if = "Option::is_none")]
    pub metadata_size: Option<u32>,
}

impl Default for ExtendedHandshake {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtendedHandshake {
    pub fn new() -> Self {
        let mut messages = HashMap::new();

        // We notify the peer: "If you want to talk ut_metadata to me, tag it with ID 1"
        messages.insert("ut_metadata".to_string(), 1);

        Self {
            messages,
            version: Some("Torry 0.1".to_string()),
            metadata_size: None,
        }
    }
}

/// The actual payload sent over the wire for BEP 9 (Metadata extraction).
#[derive(Debug, Serialize, Deserialize)]
pub struct UtMetadataMessage {
    /// 0: Request, 1: Data, 2: Reject
    #[serde(rename = "msg_type")]
    pub msg_type: u8,

    /// The index of the metadata piece (usually 16KB per piece)
    pub piece: u32,

    /// Provided by the sender on Data messages so we know when we have the whole file.
    #[serde(rename = "total_size", skip_serializing_if = "Option::is_none")]
    pub total_size: Option<u32>,
}

use napi_derive::napi;

/// The exact lifecycle states of a Torrent Session.
#[napi(string_enum)]
#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum SessionState {
    PendingMetadata,
    FetchingMetadata,
    MetadataResolved,
    Downloading,
    Seeding,
    Completed,
    Paused,
    Error,
    ErrorMissingFiles,
}

/// The core domain object representing intent and status.
#[napi(object)]
pub struct TorrentSession {
    pub info_hash: String,
    pub display_name: Option<String>,
    pub magnet_uri: String,
    pub source: Option<String>,
    pub state: SessionState,
    pub added_at: i64,
}

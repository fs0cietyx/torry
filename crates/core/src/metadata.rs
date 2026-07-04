use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawFileInfo {
    pub length: u64,
    pub path: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawInfoDict {
    pub name: String,
    #[serde(rename = "piece length")]
    pub piece_length: u32,
    #[serde(with = "serde_bytes")]
    pub pieces: Vec<u8>,

    // Single file mode
    pub length: Option<u64>,

    // Multi file mode
    pub files: Option<Vec<RawFileInfo>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileInfo {
    pub length: u64,
    pub path: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TorrentInfo {
    pub name: String,
    pub piece_length: u32,
    pub pieces: Vec<[u8; 20]>,
    pub files: Vec<FileInfo>,
    pub total_size: u64,
    pub is_multi_file: bool,
}

impl TorrentInfo {
    pub fn from_raw(raw: RawInfoDict) -> Result<Self, crate::error::TorryError> {
        let mut pieces = Vec::new();
        if !raw.pieces.len().is_multiple_of(20) {
            return Err(crate::error::TorryError::InvalidMetadata(
                "pieces length not a multiple of 20".into(),
            ));
        }
        for chunk in raw.pieces.chunks_exact(20) {
            let mut arr = [0u8; 20];
            arr.copy_from_slice(chunk);
            pieces.push(arr);
        }

        let mut files = Vec::new();
        let mut total_size = 0;
        let is_multi_file;

        if let Some(multi_files) = raw.files {
            is_multi_file = true;
            for f in multi_files {
                total_size += f.length;
                files.push(FileInfo {
                    length: f.length,
                    path: f.path,
                });
            }
        } else if let Some(length) = raw.length {
            is_multi_file = false;
            total_size = length;
            files.push(FileInfo {
                length,
                path: vec![raw.name.clone()],
            });
        } else {
            return Err(crate::error::TorryError::InvalidMetadata(
                "No length or files found".into(),
            ));
        }

        Ok(Self {
            name: raw.name,
            piece_length: raw.piece_length,
            pieces,
            files,
            total_size,
            is_multi_file,
        })
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, crate::error::TorryError> {
        let raw: RawInfoDict = serde_bencode::from_bytes(data)
            .map_err(|e| crate::error::TorryError::InvalidMetadata(e.to_string()))?;
        Self::from_raw(raw)
    }
}

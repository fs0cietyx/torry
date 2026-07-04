use std::path::PathBuf;
use tokio::sync::mpsc;

/// Commands sent FROM the TorrentManager TO the DiskIoActor.
#[derive(Debug)]
pub enum DiskCommand {
    /// Allocate file handles and pre-size them based on TorrentInfo
    AllocateFiles {
        torrent_info: crate::metadata::TorrentInfo,
    },

    /// Write the assembled .torrent metadata to the cache directory.
    WriteMetadata { data: Vec<u8> },

    /// Write a downloaded 16KB block to the target file on disk.
    WritePiece {
        piece_index: u32,
        offset: u32,
        data: Vec<u8>,
    },

    /// Read a specific block from disk for uploading.
    ReadBlock {
        piece_index: u32,
        offset: u32,
        length: u32,
        peer: std::net::SocketAddr,
    },

    /// Read an entire piece from disk, compute SHA-1, and verify it.
    VerifyHash {
        piece_index: u32,
        expected_hash: [u8; 20],
    },

    /// Read all pieces and verify them (useful for resume)
    VerifyAll { expected_hashes: Vec<[u8; 20]> },

    /// Gracefully flush and close file handles.
    Shutdown,
}

/// Events sent FROM the DiskIoActor back to the TorrentManager.
#[derive(Debug)]
pub enum DiskEvent {
    FilesAllocated,
    MetadataWritten(PathBuf),
    PieceWritten {
        piece_index: u32,
    },
    HashValid {
        piece_index: u32,
    },
    HashInvalid {
        piece_index: u32,
    },
    BlockRead {
        peer: std::net::SocketAddr,
        piece_index: u32,
        offset: u32,
        data: Vec<u8>,
    },
    VerificationComplete,
    Error(String),
}

struct OpenFile {
    mmap: Option<memmap2::MmapMut>,
    start_offset: u64,
    end_offset: u64,
}

/// The dedicated background task handling all filesystem interaction.
pub struct DiskIoActor {
    save_path: PathBuf,
    cmd_rx: mpsc::Receiver<DiskCommand>,
    manager_tx: mpsc::Sender<DiskEvent>,
    files: Vec<OpenFile>,
    piece_length: u32,
}

impl DiskIoActor {
    pub fn new(
        save_path: PathBuf,
        cmd_rx: mpsc::Receiver<DiskCommand>,
        manager_tx: mpsc::Sender<DiskEvent>,
    ) -> Self {
        Self {
            save_path,
            cmd_rx,
            manager_tx,
            files: Vec::new(),
            piece_length: 0,
        }
    }

    /// The isolated event loop for disk operations.
    pub async fn run(mut self) {
        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                DiskCommand::AllocateFiles { torrent_info } => {
                    self.piece_length = torrent_info.piece_length;
                    if let Err(e) = self.allocate_files(torrent_info) {
                        let _ = self
                            .manager_tx
                            .send(DiskEvent::Error(format!("Failed to allocate files: {}", e)))
                            .await;
                    } else {
                        let _ = self.manager_tx.send(DiskEvent::FilesAllocated).await;
                    }
                }
                DiskCommand::WritePiece {
                    piece_index,
                    offset,
                    data,
                } => {
                    let absolute_offset =
                        (piece_index as u64 * self.piece_length as u64) + offset as u64;
                    let result = tokio::task::block_in_place(|| self.write_block_to_disk(absolute_offset, &data));
                    if let Err(e) = result {
                        let _ = self.manager_tx.send(DiskEvent::Error(e.to_string())).await;
                    } else {
                        let _ = self
                            .manager_tx
                            .send(DiskEvent::PieceWritten { piece_index })
                            .await;
                    }
                }
                DiskCommand::ReadBlock {
                    piece_index,
                    offset,
                    length,
                    peer,
                } => {
                    let absolute_offset =
                        (piece_index as u64 * self.piece_length as u64) + offset as u64;
                    let result = tokio::task::block_in_place(|| self.read_block_from_disk(absolute_offset, length as usize));
                    match result {
                        Ok(data) => {
                            let _ = self
                                .manager_tx
                                .send(DiskEvent::BlockRead {
                                    peer,
                                    piece_index,
                                    offset,
                                    data,
                                })
                                .await;
                        }
                        Err(e) => {
                            let _ = self.manager_tx.send(DiskEvent::Error(e.to_string())).await;
                        }
                    }
                }
                DiskCommand::VerifyHash {
                    piece_index,
                    expected_hash,
                } => {
                    let result = tokio::task::block_in_place(|| self.verify_piece_hash(piece_index));
                    match result {
                    Ok(hash) => {
                        if hash == expected_hash {
                            let _ = self
                                .manager_tx
                                .send(DiskEvent::HashValid { piece_index })
                                .await;
                        } else {
                            let _ = self
                                .manager_tx
                                .send(DiskEvent::HashInvalid { piece_index })
                                .await;
                        }
                    }
                    Err(e) => {
                        let _ = self
                            .manager_tx
                            .send(DiskEvent::Error(format!("Hash verify err: {}", e)))
                            .await;
                    }
                    }
                },
                DiskCommand::VerifyAll { expected_hashes } => {
                    for (i, expected) in expected_hashes.into_iter().enumerate() {
                        let piece_index = i as u32;
                        let hash_res = tokio::task::block_in_place(|| self.verify_piece_hash(piece_index));
                        if let Ok(hash) = hash_res
                            && hash == expected
                        {
                            let _ = self
                                .manager_tx
                                .send(DiskEvent::HashValid { piece_index })
                                .await;
                        }
                    }
                    let _ = self.manager_tx.send(DiskEvent::VerificationComplete).await;
                }
                DiskCommand::WriteMetadata { data: _ } => {
                    let path = self.save_path.join("metadata.torrent");
                    let _ = self.manager_tx.send(DiskEvent::MetadataWritten(path)).await;
                }
                DiskCommand::Shutdown => break,
            }
        }
    }

    fn write_block_to_disk(
        &mut self,
        absolute_offset: u64,
        data: &[u8],
    ) -> Result<(), std::io::Error> {
        let mut remaining_data = data;
        let mut current_offset = absolute_offset;

        for file in &mut self.files {
            if remaining_data.is_empty() {
                break;
            }

            if current_offset >= file.start_offset && current_offset < file.end_offset {
                let offset_in_file = current_offset - file.start_offset;
                let available_in_file = file.end_offset - current_offset;
                let write_len =
                    std::cmp::min(remaining_data.len() as u64, available_in_file) as usize;

                if let Some(mmap) = &mut file.mmap {
                    let start = offset_in_file as usize;
                    let end = start + write_len;
                    mmap[start..end].copy_from_slice(&remaining_data[..write_len]);
                }

                remaining_data = &remaining_data[write_len..];
                current_offset += write_len as u64;
            }
        }

        if !remaining_data.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Tried to write past end of all files",
            ));
        }
        Ok(())
    }

    fn read_block_from_disk(
        &mut self,
        absolute_offset: u64,
        length: usize,
    ) -> Result<Vec<u8>, std::io::Error> {
        let mut buf = vec![0u8; length];
        let mut remaining_len = length;
        let mut current_offset = absolute_offset;
        let mut buf_offset = 0;

        for file in &mut self.files {
            if remaining_len == 0 {
                break;
            }

            if current_offset >= file.start_offset && current_offset < file.end_offset {
                let offset_in_file = current_offset - file.start_offset;
                let available_in_file = file.end_offset - current_offset;
                let read_len = std::cmp::min(remaining_len as u64, available_in_file) as usize;

                if let Some(mmap) = &file.mmap {
                    let start = offset_in_file as usize;
                    let end = start + read_len;
                    buf[buf_offset..buf_offset + read_len].copy_from_slice(&mmap[start..end]);
                }

                remaining_len -= read_len;
                current_offset += read_len as u64;
                buf_offset += read_len;
            }
        }

        if remaining_len > 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Tried to read past end of all files",
            ));
        }
        Ok(buf)
    }

    fn verify_piece_hash(&mut self, piece_index: u32) -> Result<[u8; 20], std::io::Error> {
        let absolute_offset = piece_index as u64 * self.piece_length as u64;
        let mut remaining_len = self.piece_length as u64;

        // Handle the last piece which might be smaller
        let total_size = self.files.last().map(|f| f.end_offset).unwrap_or(0);
        if absolute_offset + remaining_len > total_size {
            remaining_len = total_size - absolute_offset;
        }

        let mut current_offset = absolute_offset;
        let mut hasher = sha1_smol::Sha1::new();
        let _buf = vec![0u8; 16384]; // 16KB chunks

        for file in &mut self.files {
            if remaining_len == 0 {
                break;
            }

            if current_offset >= file.start_offset && current_offset < file.end_offset {
                let offset_in_file = current_offset - file.start_offset;
                let available_in_file = file.end_offset - current_offset;
                let read_len = std::cmp::min(remaining_len, available_in_file);

                if let Some(mmap) = &file.mmap {
                    let start = offset_in_file as usize;
                    let end = start + read_len as usize;
                    hasher.update(&mmap[start..end]);
                }

                remaining_len -= read_len;
                current_offset += read_len;
            }
        }

        Ok(hasher.digest().bytes())
    }

    fn allocate_files(&mut self, info: crate::metadata::TorrentInfo) -> Result<(), std::io::Error> {
        std::fs::create_dir_all(&self.save_path)?;

        let base_path = if info.is_multi_file {
            let p = self.save_path.join(&info.name);
            std::fs::create_dir_all(&p)?;
            p
        } else {
            self.save_path.clone()
        };

        let mut current_offset = 0;

        for file_info in info.files {
            let mut file_path = base_path.clone();
            for component in &file_info.path {
                file_path.push(component);
            }

            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(&file_path)?;

            // Pre-allocate the space on disk
            file.set_len(file_info.length)?;

            let mmap = if file_info.length > 0 {
                Some(unsafe { memmap2::MmapMut::map_mut(&file)? })
            } else {
                None
            };

            let end_offset = current_offset + file_info.length;
            self.files.push(OpenFile {
                mmap,
                start_offset: current_offset,
                end_offset,
            });
            current_offset = end_offset;
        }

        Ok(())
    }
}

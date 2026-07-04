use bytes::{BufMut, BytesMut};

pub const BITTORRENT_PROTOCOL: &[u8; 19] = b"BitTorrent protocol";

/// The standard 68-byte BitTorrent handshake.
pub struct Handshake {
    pub reserved: [u8; 8],
    pub info_hash: [u8; 20],
    pub peer_id: [u8; 20],
}

impl Handshake {
    pub fn new(info_hash: [u8; 20], peer_id: [u8; 20]) -> Self {
        let mut reserved = [0u8; 8];

        // BEP 10: Enable the Extension Protocol.
        // The 43rd bit (counting from left) must be 1.
        // This lands exactly on Byte 5, Bit 0x10.
        reserved[5] |= 0x10;

        Self {
            reserved,
            info_hash,
            peer_id,
        }
    }

    /// Packs the handshake into a high-performance byte buffer for the TCP socket.
    pub fn to_bytes(&self) -> BytesMut {
        let mut buf = BytesMut::with_capacity(68);
        buf.put_u8(19);
        buf.put_slice(BITTORRENT_PROTOCOL);
        buf.put_slice(&self.reserved);
        buf.put_slice(&self.info_hash);
        buf.put_slice(&self.peer_id);
        buf
    }
}

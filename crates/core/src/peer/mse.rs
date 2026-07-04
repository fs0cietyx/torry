use num_bigint::BigUint;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// MSE/PE 768-bit Oakley Group 2 prime
const P_HEX: &str = "FFFFFFFFFFFFFFFFC90FDAA22168C234C4C6628B80DC1CD129024E088A67CC74020BBEA63B139B22514A08798E3404DDEF9519B3CD3A431B302B0A6DF25F14374FE1356D6D51C245E485B576625E7EC6F44C42E9A63A36210000000000090563";

const VC: [u8; 8] = [0u8; 8];
const CRYPTO_PLAINTEXT: u32 = 0x01;
const CRYPTO_RC4: u32 = 0x02;

// ─────────────────────────────────────────────
//  RC4 Stream Cipher
// ─────────────────────────────────────────────
pub struct Rc4 {
    s: [u8; 256],
    i: u8,
    j: u8,
}

impl Rc4 {
    fn new(key: &[u8]) -> Self {
        let mut s = [0u8; 256];
        for (k, item) in s.iter_mut().enumerate() {
            *item = k as u8;
        }
        let mut j: u8 = 0;
        for k in 0..256 {
            j = j.wrapping_add(s[k]).wrapping_add(key[k % key.len()]);
            s.swap(k, j as usize);
        }
        Rc4 { s, i: 0, j: 0 }
    }

    pub fn process(&mut self, data: &mut [u8]) {
        for byte in data.iter_mut() {
            self.i = self.i.wrapping_add(1);
            self.j = self.j.wrapping_add(self.s[self.i as usize]);
            self.s.swap(self.i as usize, self.j as usize);
            let k = self.s[self.s[self.i as usize].wrapping_add(self.s[self.j as usize]) as usize];
            *byte ^= k;
        }
    }

    fn discard(&mut self, n: usize) {
        let mut buf = vec![0u8; n];
        self.process(&mut buf);
    }
}

// ─────────────────────────────────────────────
//  SHA1 Helper
// ─────────────────────────────────────────────
fn sha1_multi(parts: &[&[u8]]) -> [u8; 20] {
    let mut h = sha1_smol::Sha1::new();
    for p in parts {
        h.update(p);
    }
    h.digest().bytes()
}

// ─────────────────────────────────────────────
//  Encrypted Peer Stream Wrapper
// ─────────────────────────────────────────────

/// Wraps TcpStream with optional RC4 encryption. All reads/writes are transparently
/// encrypted/decrypted if MSE negotiated RC4, or passed through if plaintext was selected.
pub struct PeerStream<S> {
    inner: S,
    encrypt: Option<Rc4>,
    decrypt: Option<Rc4>,
}

impl<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin> PeerStream<S> {
    /// Wrap with no encryption (plaintext fallback).
    pub fn plaintext(stream: S) -> Self {
        Self {
            inner: stream,
            encrypt: None,
            decrypt: None,
        }
    }

    fn encrypted(stream: S, encrypt: Rc4, decrypt: Rc4) -> Self {
        Self {
            inner: stream,
            encrypt: Some(encrypt),
            decrypt: Some(decrypt),
        }
    }

    /// Write data, encrypting if MSE is active.
    pub async fn write_all(&mut self, data: &[u8]) -> Result<(), std::io::Error> {
        use tokio::io::AsyncWriteExt;
        if let Some(ref mut rc4) = self.encrypt {
            let mut buf = data.to_vec();
            rc4.process(&mut buf);
            self.inner.write_all(&buf).await
        } else {
            self.inner.write_all(data).await
        }
    }

    /// Read exactly `buf.len()` bytes, decrypting if MSE is active.
    pub async fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), std::io::Error> {
        use tokio::io::AsyncReadExt;
        self.inner.read_exact(buf).await?;
        if let Some(ref mut rc4) = self.decrypt {
            rc4.process(buf);
        }
        Ok(())
    }

    /// Access the underlying stream (for socket options etc.)
    pub fn tcp_stream(&self) -> &S {
        &self.inner
    }
}

// ─────────────────────────────────────────────
//  MSE Handshake (Initiator)
// ─────────────────────────────────────────────

/// Perform MSE handshake as the connection initiator.
/// Takes ownership of the TcpStream and returns a PeerStream wrapper.
/// Takes ownership of the stream and returns a PeerStream wrapper.
///
/// If the peer doesn't support MSE, this will fail and the caller should
/// fall back to a plaintext PeerStream.
pub async fn mse_handshake<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
    mut stream: S,
    info_hash: &[u8; 20],
) -> Result<PeerStream<S>, (S, std::io::Error)> {
    match mse_handshake_inner(&mut stream, info_hash).await {
        Ok((encrypt, decrypt)) => Ok(PeerStream::encrypted(stream, encrypt, decrypt)),
        Err(e) => Err((stream, e)),
    }
}

async fn mse_handshake_inner<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
    stream: &mut S,
    info_hash: &[u8; 20],
) -> Result<(Rc4, Rc4), std::io::Error> {
    let p = BigUint::parse_bytes(P_HEX.as_bytes(), 16)
        .ok_or_else(|| std::io::Error::other("MSE: bad prime"))?;
    let g = BigUint::from(2u32);

    // ── Step 1: DH Key Exchange ──
    let a_bytes: [u8; 20] = rand::random();
    let a = BigUint::from_bytes_be(&a_bytes);
    
    let a_clone = a.clone();
    let p_clone = p.clone();
    let g_clone = g.clone();
    let ya = tokio::task::spawn_blocking(move || g_clone.modpow(&a_clone, &p_clone))
        .await
        .map_err(|_| std::io::Error::other("MSE task panicked"))?;

    // Send Ya (96 bytes, zero-padded to 768 bits)
    let ya_raw = ya.to_bytes_be();
    let mut ya_padded = vec![0u8; 96];
    let offset = 96usize.saturating_sub(ya_raw.len());
    ya_padded[offset..].copy_from_slice(&ya_raw);
    stream.write_all(&ya_padded).await?;

    // Read Yb (96 bytes)
    let mut yb_buf = [0u8; 96];
    stream.read_exact(&mut yb_buf).await?;
    let yb = BigUint::from_bytes_be(&yb_buf);

    // Shared secret S = Yb^a mod p
    let s = tokio::task::spawn_blocking(move || yb.modpow(&a, &p))
        .await
        .map_err(|_| std::io::Error::other("MSE task panicked"))?;
    let s_bytes = s.to_bytes_be();
    let skey: &[u8] = info_hash;

    // ── Step 2: Send crypto negotiation ──

    // req1 = HASH('req1', S)
    let req1 = sha1_multi(&[b"req1", &s_bytes]);

    // req2 = HASH('req2', SKEY) XOR HASH('req3', S)
    let h2 = sha1_multi(&[b"req2", skey]);
    let h3 = sha1_multi(&[b"req3", &s_bytes]);
    let mut req2 = [0u8; 20];
    for i in 0..20 {
        req2[i] = h2[i] ^ h3[i];
    }

    // Init RC4 ciphers (discard 1024 bytes per spec)
    let enc_key = sha1_multi(&[b"keyA", &s_bytes, skey]);
    let dec_key = sha1_multi(&[b"keyB", &s_bytes, skey]);
    let mut encrypt = Rc4::new(&enc_key);
    encrypt.discard(1024);

    // Encrypted payload: VC(8) + crypto_provide(4) + len_padC(2) + len_IA(2) = 16 bytes
    let crypto_provide: u32 = CRYPTO_PLAINTEXT | CRYPTO_RC4;
    let mut payload = Vec::with_capacity(16);
    payload.extend_from_slice(&VC);
    payload.extend_from_slice(&crypto_provide.to_be_bytes());
    payload.extend_from_slice(&0u16.to_be_bytes()); // PadC len = 0
    payload.extend_from_slice(&0u16.to_be_bytes()); // IA len = 0
    encrypt.process(&mut payload);

    let mut msg = Vec::with_capacity(56);
    msg.extend_from_slice(&req1);
    msg.extend_from_slice(&req2);
    msg.extend_from_slice(&payload);
    stream.write_all(&msg).await?;

    // ── Step 3: Read responder's reply ──
    // Responder sends PadB (0-512 plaintext bytes) then ENCRYPT(VC + crypto_select + pad_d_len + PadD)
    // We decrypt sequentially through the cipher and scan for the VC pattern.

    let mut decrypt = Rc4::new(&dec_key);
    decrypt.discard(1024);

    // Read and decrypt one byte at a time, scanning for 8 consecutive zero bytes (VC)
    let mut vc_window = [0xFFu8; 8]; // init to non-zero
    let mut window_pos = 0usize;
    let mut total_read = 0usize;

    loop {
        if total_read > 520 {
            // 512 max pad + 8 VC
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "MSE: VC not found",
            ));
        }
        let mut byte = [0u8; 1];
        stream.read_exact(&mut byte).await?;
        decrypt.process(&mut byte);
        total_read += 1;

        vc_window[window_pos % 8] = byte[0];
        window_pos += 1;

        if window_pos >= 8 {
            // Check if the circular buffer contains all zeros
            let all_zero = (0..8).all(|k| vc_window[(window_pos - 8 + k) % 8] == 0);
            if all_zero {
                break; // Found VC!
            }
        }
    }

    // Read crypto_select(4) + pad_d_len(2) = 6 bytes
    let mut header = [0u8; 6];
    stream.read_exact(&mut header).await?;
    decrypt.process(&mut header);

    let crypto_select = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
    let pad_d_len = u16::from_be_bytes([header[4], header[5]]) as usize;

    // Read and discard PadD
    if pad_d_len > 0 && pad_d_len <= 512 {
        let mut pad_d = vec![0u8; pad_d_len];
        stream.read_exact(&mut pad_d).await?;
        decrypt.process(&mut pad_d);
    }

    // Return ciphers based on what the responder selected
    if crypto_select & CRYPTO_RC4 != 0 {
        Ok((encrypt, decrypt))
    } else if crypto_select & CRYPTO_PLAINTEXT != 0 {
        // Responder chose plaintext — no encryption needed going forward
        // But we still needed MSE handshake to get past DPI
        Err(std::io::Error::other("MSE: plaintext selected"))
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "MSE: No acceptable crypto",
        ))
    }
}

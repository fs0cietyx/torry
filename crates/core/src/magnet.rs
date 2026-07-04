use napi_derive::napi;

/// The structured object returned to TypeScript via NAPI-RS.
/// `#[napi(object)]` tells the macro to convert this Rust struct into a plain JS Object.
#[napi(object)]
pub struct ParsedMagnet {
    pub info_hash: String,
    pub display_name: Option<String>,
    pub trackers: Vec<String>,
}

/// Parses a raw magnet URI string into a validated structured object.
#[napi]
pub fn parse_magnet_uri(uri: String) -> napi::Result<ParsedMagnet> {
    // 1. Validate Scheme
    if !uri.starts_with("magnet:?") {
        return Err(napi::Error::from_reason(
            "Invalid scheme. Must start with 'magnet:?'",
        ));
    }

    let mut info_hash = None;
    let mut display_name = None;
    let mut trackers = Vec::new();

    // 2. Strip 'magnet:?' and split by '&'
    let query = &uri[8..];

    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or("");
        let val = parts.next().unwrap_or("");

        match key {
            "xt" => {
                // BitTorrent Info Hash
                if let Some(stripped) = val.strip_prefix("urn:btih:") {
                    info_hash = Some(stripped.to_string());
                }
            }
            "dn" => {
                // Basic decode for display name (+ to space)
                display_name = Some(val.replace("+", " "));
            }
            "tr" => {
                // Trackers are URL-encoded. MVP naive decode:
                let decoded_tracker = val.replace("%3A", ":").replace("%2F", "/");
                trackers.push(decoded_tracker);
            }
            _ => {} // Ignore other parameters like xl (exact length), ws (web seed), etc.
        }
    }

    // 3. Validation Rules
    let hash = info_hash.ok_or_else(|| {
        napi::Error::from_reason("Magnet URI must contain a BitTorrent Info Hash (xt=urn:btih:...)")
    })?;

    // Hex hashes are 40 chars, Base32 hashes are 32 chars.
    if hash.len() != 40 && hash.len() != 32 {
        return Err(napi::Error::from_reason(
            "Info Hash must be exactly 40 (hex) or 32 (base32) characters",
        ));
    }

    Ok(ParsedMagnet {
        info_hash: hash,
        display_name,
        trackers,
    })
}

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Pes,
    Dst,
    Exp,
    Jef,
    Vp3,
}

/// Detect format from file content (magic bytes).
pub fn detect_from_bytes(data: &[u8]) -> Option<Format> {
    if data.len() >= 4 && &data[0..4] == b"#PES" {
        return Some(Format::Pes);
    }
    if data.len() >= 5 && &data[0..5] == b"%vsm%" {
        return Some(Format::Vp3);
    }
    None
}

/// Detect format from file extension.
pub fn detect_from_extension(path: &Path) -> Option<Format> {
    let ext = path.extension()?.to_str()?;
    match ext.to_ascii_lowercase().as_str() {
        "pes" => Some(Format::Pes),
        "dst" => Some(Format::Dst),
        "exp" => Some(Format::Exp),
        "jef" => Some(Format::Jef),
        "vp3" => Some(Format::Vp3),
        _ => None,
    }
}

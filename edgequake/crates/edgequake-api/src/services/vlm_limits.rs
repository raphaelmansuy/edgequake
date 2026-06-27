//! VLM image size gates (LightRAG `VLM_MIN_IMAGE_PIXEL` / `VLM_MAX_IMAGE_BYTES` parity).

const DEFAULT_MIN_PIXELS: u64 = 64;
const DEFAULT_MAX_BYTES: usize = 5 * 1024 * 1024;

/// Minimum pixel count (width × height) for VLM analysis.
pub fn vlm_min_image_pixels() -> u64 {
    std::env::var("VLM_MIN_IMAGE_PIXEL")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_MIN_PIXELS)
}

/// Maximum raw image bytes sent to VLM.
pub fn vlm_max_image_bytes() -> usize {
    std::env::var("VLM_MAX_IMAGE_BYTES")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n| n >= 256 * 1024)
        .unwrap_or(DEFAULT_MAX_BYTES)
}

/// Best-effort width × height from raw bytes. Returns `None` when unknown (fail-closed upstream).
pub fn probe_image_dimensions(bytes: &[u8], mime_type: &str) -> Option<(u32, u32)> {
    let mime = mime_type.to_ascii_lowercase();
    if mime.contains("png") || bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        return png_dimensions(bytes);
    }
    if mime.contains("jpeg") || mime.contains("jpg") || bytes.starts_with(&[0xFF, 0xD8]) {
        return jpeg_dimensions(bytes);
    }
    if mime.contains("webp") || bytes.starts_with(b"RIFF") {
        return webp_dimensions(bytes);
    }
    None
}

fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || bytes[0..8] != [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
        return None;
    }
    if &bytes[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((width, height))
}

fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 4 || bytes[0..2] != [0xFF, 0xD8] {
        return None;
    }
    let mut i = 2usize;
    while i + 9 < bytes.len() {
        if bytes[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = bytes[i + 1];
        if matches!(marker, 0xC0..=0xC3) {
            let height = u16::from_be_bytes(bytes[i + 5..i + 7].try_into().ok()?) as u32;
            let width = u16::from_be_bytes(bytes[i + 7..i + 9].try_into().ok()?) as u32;
            return Some((width, height));
        }
        let seg_len = u16::from_be_bytes(bytes[i + 2..i + 4].try_into().ok()?);
        if seg_len < 2 {
            return None;
        }
        i = i.saturating_add(2 + seg_len as usize);
    }
    None
}

fn webp_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 30 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
        return None;
    }
    if &bytes[12..16] == b"VP8X" && bytes.len() >= 30 {
        let w =
            1 + u32::from(bytes[24]) + (u32::from(bytes[25]) << 8) + (u32::from(bytes[26]) << 16);
        let h =
            1 + u32::from(bytes[27]) + (u32::from(bytes[28]) << 8) + (u32::from(bytes[29]) << 16);
        return Some((w, h));
    }
    if &bytes[12..16] == b"VP8 " && bytes.len() >= 30 {
        let width = u16::from_le_bytes(bytes[26..28].try_into().ok()?) as u32 & 0x3FFF;
        let height = u16::from_le_bytes(bytes[28..30].try_into().ok()?) as u32 & 0x3FFF;
        return Some((width, height));
    }
    None
}

/// Validate image payload before VLM call; returns human-readable error.
pub fn validate_image_for_vlm(bytes: &[u8], width: u32, height: u32) -> Result<(), String> {
    if bytes.is_empty() {
        return Err("empty image payload".into());
    }
    if bytes.len() > vlm_max_image_bytes() {
        return Err(format!(
            "image exceeds VLM_MAX_IMAGE_BYTES ({} > {})",
            bytes.len(),
            vlm_max_image_bytes()
        ));
    }
    if width == 0 || height == 0 {
        return Err("unknown image dimensions (fail-closed)".into());
    }
    let pixels = u64::from(width) * u64::from(height);
    if pixels < vlm_min_image_pixels() {
        return Err(format!(
            "image below VLM_MIN_IMAGE_PIXEL ({pixels} < {})",
            vlm_min_image_pixels()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_lightrag_spec() {
        std::env::remove_var("VLM_MIN_IMAGE_PIXEL");
        assert_eq!(vlm_min_image_pixels(), 64);
        assert_eq!(vlm_max_image_bytes(), 5 * 1024 * 1024);
    }

    #[test]
    fn probe_png_1x1_dimensions() {
        let png: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89,
        ];
        assert_eq!(probe_image_dimensions(png, "image/png"), Some((1, 1)));
        assert!(validate_image_for_vlm(png, 1, 1).is_err());
    }

    #[test]
    fn fail_closed_on_zero_dimensions() {
        let png: &[u8] = &[0x89, 0x50, 0x4E, 0x47];
        assert!(validate_image_for_vlm(png, 0, 0).is_err());
    }
}

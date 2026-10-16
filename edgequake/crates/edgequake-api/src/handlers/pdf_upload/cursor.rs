//! PDF list cursor.
//!
//! A page number stays an offset. `k1.created.desc.{hex}.{uuid}` is a keyset:
//! the next request requires `(created_at, pdf_id)` so the index stops at the page.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Decode a PDF list cursor. Empty or unrecognized text is not a keyset.
pub(crate) fn parse_pdf_cursor(cursor: Option<&str>) -> Option<(DateTime<Utc>, Uuid)> {
    let raw = cursor.map(str::trim).filter(|s| !s.is_empty())?;
    let mut parts = raw.split('.');
    if parts.next()? != "k1" || parts.next()? != "created" || parts.next()? != "desc" {
        return None;
    }
    let at = hex_decode(parts.next()?)?;
    let id = Uuid::parse_str(parts.next()?).ok()?;
    if parts.next().is_some() {
        return None;
    }
    let at = DateTime::parse_from_rfc3339(&at).ok()?.with_timezone(&Utc);
    Some((at, id))
}

pub(crate) fn encode_pdf_cursor(at: DateTime<Utc>, id: Uuid) -> String {
    format!(
        "k1.created.desc.{}.{id}",
        hex_encode(at.to_rfc3339().as_bytes())
    )
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0xf) as usize] as char);
    }
    out
}

fn hex_decode(text: &str) -> Option<String> {
    if text.is_empty() || !text.len().is_multiple_of(2) {
        return None;
    }
    let mut bytes = Vec::with_capacity(text.len() / 2);
    let chars: Vec<u8> = text.bytes().collect();
    for pair in chars.chunks(2) {
        let hi = from_hex(pair[0])?;
        let lo = from_hex(pair[1])?;
        bytes.push((hi << 4) | lo);
    }
    String::from_utf8(bytes).ok()
}

fn from_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdf_cursor_round_trips_and_rejects_a_page_number() {
        let at = DateTime::parse_from_rfc3339("2026-03-01T00:01:40+00:00")
            .unwrap()
            .with_timezone(&Utc);
        let id = Uuid::parse_str("f332f916-68d0-4b52-ac94-8a8cbe5b2488").unwrap();
        let encoded = encode_pdf_cursor(at, id);
        assert_eq!(parse_pdf_cursor(Some(&encoded)), Some((at, id)));
        assert_eq!(parse_pdf_cursor(Some("2")), None);
        assert_eq!(parse_pdf_cursor(None), None);
        assert_eq!(parse_pdf_cursor(Some("k1.created.asc.00")), None);
    }
}

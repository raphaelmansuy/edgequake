//! Conversation and message list cursors (SPEC-141).
//!
//! A decimal cursor is still an offset, so an old client keeps working.
//! `next_cursor` is a keyset. Passing that value back requires
//! `(sort_column, id)` to move, and the index stops at the page instead of
//! walking every skipped row.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::types::PaginationMeta;

/// Parsed `cursor` query value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ParsedCursor {
    Offset(usize),
    AfterTime {
        field: String,
        desc: bool,
        at: DateTime<Utc>,
        id: Uuid,
    },
    AfterTitle {
        desc: bool,
        title: String,
        id: Uuid,
    },
}

/// Parse `cursor`. A decimal is an offset. A `k1` token is a keyset.
/// Missing or unrecognized text starts at the first row.
pub(crate) fn parse_list_cursor(cursor: Option<&str>) -> ParsedCursor {
    let Some(raw) = cursor.map(str::trim).filter(|s| !s.is_empty()) else {
        return ParsedCursor::Offset(0);
    };
    if let Ok(offset) = raw.parse::<usize>() {
        return ParsedCursor::Offset(offset);
    }
    parse_keyset(raw).unwrap_or(ParsedCursor::Offset(0))
}

fn parse_keyset(raw: &str) -> Option<ParsedCursor> {
    let mut parts = raw.split('.');
    if parts.next()? != "k1" {
        return None;
    }
    let field = parts.next()?.to_string();
    let desc = match parts.next()? {
        "desc" => true,
        "asc" => false,
        _ => return None,
    };
    let value = hex_decode(parts.next()?)?;
    let id = Uuid::parse_str(parts.next()?).ok()?;
    if parts.next().is_some() || field.is_empty() {
        return None;
    }
    if field == "title" {
        return Some(ParsedCursor::AfterTitle {
            desc,
            title: value,
            id,
        });
    }
    let at = DateTime::parse_from_rfc3339(&value)
        .ok()?
        .with_timezone(&Utc);
    Some(ParsedCursor::AfterTime {
        field,
        desc,
        at,
        id,
    })
}

pub(crate) fn encode_time_cursor(field: &str, desc: bool, at: DateTime<Utc>, id: Uuid) -> String {
    encode_cursor(field, desc, &at.to_rfc3339(), id)
}

pub(crate) fn encode_title_cursor(desc: bool, title: &str, id: Uuid) -> String {
    encode_cursor("title", desc, title, id)
}

fn encode_cursor(field: &str, desc: bool, value: &str, id: Uuid) -> String {
    let dir = if desc { "desc" } else { "asc" };
    format!("k1.{field}.{dir}.{}.{id}", hex_encode(value.as_bytes()))
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

/// Honest `has_more` / cursors for a decimal offset page.
///
/// `has_more` is `offset + taken < total`. Callers replace `next_cursor`
/// with a keyset when the page has a last row, so the following request
/// does not repeat the offset walk.
pub(crate) fn pagination_meta(
    offset: usize,
    limit: usize,
    taken: usize,
    total: usize,
) -> PaginationMeta {
    let next_offset = offset.saturating_add(taken);
    let has_more = next_offset < total;
    PaginationMeta {
        next_cursor: has_more.then(|| next_offset.to_string()),
        prev_cursor: (offset > 0).then(|| offset.saturating_sub(limit).to_string()),
        total: Some(total),
        has_more,
    }
}

/// Keyset page. `has_more` comes from asking for one extra row, because the
/// numeric position of this page is unknown.
pub(crate) fn keyset_page_meta(
    has_more: bool,
    total: usize,
    next_cursor: Option<String>,
) -> PaginationMeta {
    PaginationMeta {
        next_cursor,
        prev_cursor: None,
        total: Some(total),
        has_more,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn garbage_cursor_is_offset_zero() {
        assert_eq!(parse_list_cursor(None), ParsedCursor::Offset(0));
        assert_eq!(parse_list_cursor(Some("")), ParsedCursor::Offset(0));
        assert_eq!(parse_list_cursor(Some("abc")), ParsedCursor::Offset(0));
        assert_eq!(parse_list_cursor(Some("20")), ParsedCursor::Offset(20));
    }

    #[test]
    fn time_keyset_round_trips() {
        let at = DateTime::parse_from_rfc3339("2026-01-01T00:00:41Z")
            .unwrap()
            .with_timezone(&Utc);
        let id = Uuid::parse_str("4d89c699-3a3c-4592-9046-5cdded6910c7").unwrap();
        let cursor = encode_time_cursor("updated_at", true, at, id);
        assert!(cursor.starts_with("k1.updated_at.desc."));
        assert_eq!(
            parse_list_cursor(Some(&cursor)),
            ParsedCursor::AfterTime {
                field: "updated_at".into(),
                desc: true,
                at,
                id,
            }
        );
    }

    #[test]
    fn title_keyset_round_trips() {
        let id = Uuid::nil();
        let cursor = encode_title_cursor(false, "Chat 19", id);
        assert_eq!(
            parse_list_cursor(Some(&cursor)),
            ParsedCursor::AfterTitle {
                desc: false,
                title: "Chat 19".into(),
                id,
            }
        );
    }

    #[test]
    fn last_page_has_no_next_cursor() {
        let meta = pagination_meta(20, 20, 5, 25);
        assert!(!meta.has_more);
        assert!(meta.next_cursor.is_none());
        assert_eq!(meta.prev_cursor.as_deref(), Some("0"));
    }

    #[test]
    fn full_page_with_remainder_sets_next_cursor() {
        let meta = pagination_meta(0, 20, 20, 25);
        assert!(meta.has_more);
        assert_eq!(meta.next_cursor.as_deref(), Some("20"));
    }
}

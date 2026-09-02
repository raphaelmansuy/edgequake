//! Observation I/O policy (SPEC-145 / LAW-145-1..6).
//!
//! ## SOLID
//! - **S**: Clamp, redact, and classify only — no exporter I/O.
//! - **O**: New classes without rewriting call sites.
//! - **D**: Call sites choose [`IoPolicy`]; they never invent byte caps.

use std::sync::OnceLock;

use crate::utf8_truncate::utf8_prefix;

/// Format role-labeled chat turns for Complete observation I/O (LAW-145-1).
///
/// Each turn is `(role_label, content, image_count)`. Image binaries are noted
/// as a count so I/O stays UTF-8 without dumping base64.
pub fn format_llm_chat_turns_for_observation(
    turns: impl IntoIterator<Item = (impl AsRef<str>, impl AsRef<str>, usize)>,
) -> String {
    let mut parts = Vec::new();
    for (role, content, image_count) in turns {
        let mut body = content.as_ref().trim().to_string();
        if image_count > 0 {
            if !body.is_empty() {
                body.push('\n');
            }
            body.push_str(&format!("[{image_count} image(s) attached]"));
        }
        parts.push(format!("{}: {body}", role.as_ref()));
    }
    parts.join("\n\n")
}

/// Default per-field safety ceiling for [`IoPolicy::Complete`].
///
/// `0` means **unlimited** (never product-truncate LLM I/O). Operators may set
/// `EDGEQUAKE_LANGFUSE_IO_MAX_BYTES` to a positive byte budget if they need a
/// hard ceiling (honest `io_complete=false` when hit).
pub const DEFAULT_LANGFUSE_IO_MAX_BYTES: usize = 0;

/// Legacy preview budget (SPEC-124). Kept for [`IoPolicy::Preview`] default and
/// retriever snippet helpers — **not** applied to Complete generation I/O.
pub const OBSERVATION_IO_PREVIEW_CHARS: usize = 512;

/// Ingest document content preview budget (bytes).
pub const INGEST_CONTENT_PREVIEW_BYTES: usize = 256;

/// Retriever nested preview snippet (bytes).
pub const RETRIEVAL_PREVIEW_BYTES: usize = 128;

/// Synthetic tail marker for unfakable proofs (LAW-145-8 / 10). No personal data.
pub const MARKER_TAIL_COMPLETE: &str = "__EQ_IO_COMPLETE_TAIL__";

/// How observation Input/Output strings are prepared before dual-write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoPolicy {
    /// Full model I/O: redact secrets; optional env ceiling with honest metadata.
    /// Default: **no length truncation** (LAW-145-1).
    Complete,
    /// Compact JSON / stats — emit as-is (already short).
    Structured,
    /// Explicit byte preview with ellipsis (ingest content only).
    Preview { max_bytes: usize },
}

/// Result of applying a policy to one I/O field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedIo {
    pub text: String,
    /// `false` when Complete hit the safety ceiling (LAW-145-6).
    pub complete: bool,
    /// Original byte length before clamp (after redaction).
    pub io_bytes: usize,
}

/// Resolve `EDGEQUAKE_LANGFUSE_IO_MAX_BYTES` once (env-only, SPEC-124 pattern).
///
/// - Unset / empty / `0` → unlimited (`0`)
/// - Positive integer → per-field clamp for Complete class
pub fn langfuse_io_max_bytes() -> usize {
    static MAX: OnceLock<usize> = OnceLock::new();
    *MAX.get_or_init(|| {
        std::env::var("EDGEQUAKE_LANGFUSE_IO_MAX_BYTES")
            .ok()
            .and_then(|v| {
                let t = v.trim().trim_matches(|c| c == '"' || c == '\'');
                if t.is_empty() || t == "0" {
                    return Some(0);
                }
                t.parse::<usize>().ok()
            })
            .unwrap_or(DEFAULT_LANGFUSE_IO_MAX_BYTES)
    })
}

/// Apply [`IoPolicy`] to one observation field (LAW-145-1..6).
pub fn prepare_observation_io(raw: &str, policy: IoPolicy) -> PreparedIo {
    match policy {
        IoPolicy::Structured => PreparedIo {
            io_bytes: raw.len(),
            complete: true,
            text: raw.to_string(),
        },
        IoPolicy::Preview { max_bytes } => {
            let text = preview_bytes(raw, max_bytes);
            PreparedIo {
                io_bytes: raw.len(),
                complete: raw.len() <= max_bytes,
                text,
            }
        }
        IoPolicy::Complete => prepare_complete_io(raw, langfuse_io_max_bytes()),
    }
}

/// Complete-class prepare with an explicit ceiling (tests + env SSOT).
///
/// `max_bytes == 0` means unlimited — never truncate (secrets still redacted).
pub fn prepare_complete_io(raw: &str, max_bytes: usize) -> PreparedIo {
    let redacted = redact_secrets(raw);
    let io_bytes = redacted.len();
    if max_bytes == 0 || io_bytes <= max_bytes {
        PreparedIo {
            text: redacted,
            complete: true,
            io_bytes,
        }
    } else {
        PreparedIo {
            text: utf8_prefix(&redacted, max_bytes).to_string(),
            complete: false,
            io_bytes,
        }
    }
}

/// UTF-8-safe byte preview with ellipsis (LAW-145-5: bytes only).
pub fn preview_bytes(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    format!("{}…", utf8_prefix(s, max_bytes))
}

/// Redact secret-shaped substrings before Complete emit (LAW-145-2).
///
/// Prefer key-shaped patterns to limit false positives on prose `sk-`.
pub fn redact_secrets(s: &str) -> String {
    let mut out = redact_bearer(s);
    // Longer prefixes first; bare `sk-` must not re-match `sk-lf-` / `sk-proj-`.
    out = redact_prefixed_keys(&out, "sk-lf-");
    out = redact_prefixed_keys(&out, "sk-proj-");
    out = redact_bare_sk(&out);
    out
}

/// Redact `sk-…` keys that are not already `sk-lf-` / `sk-proj-`.
fn redact_bare_sk(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    loop {
        if let Some(pos) = rest.find("sk-") {
            out.push_str(&rest[..pos]);
            let tail = &rest[pos..];
            if tail.starts_with("sk-lf-") || tail.starts_with("sk-proj-") {
                // Leave known longer forms (already redacted or pending) intact.
                out.push_str("sk-");
                rest = &rest[pos + 3..];
                continue;
            }
            out.push_str("sk-[REDACTED]");
            let after = pos + 3;
            let token = &rest[after..];
            let skip = token
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                .map(|c| c.len_utf8())
                .sum::<usize>();
            rest = &token[skip..];
        } else {
            out.push_str(rest);
            break;
        }
    }
    out
}

fn redact_bearer(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    loop {
        let lower = rest.to_ascii_lowercase();
        if let Some(pos) = lower.find("bearer ") {
            out.push_str(&rest[..pos]);
            out.push_str("Bearer [REDACTED]");
            let after = pos + "bearer ".len();
            let tail = &rest[after..];
            let skip = tail.find(|c: char| c.is_whitespace()).unwrap_or(tail.len());
            rest = &tail[skip..];
        } else {
            out.push_str(rest);
            break;
        }
    }
    out
}

fn redact_prefixed_keys(s: &str, prefix: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    loop {
        if let Some(pos) = rest.find(prefix) {
            out.push_str(&rest[..pos]);
            out.push_str(prefix);
            out.push_str("[REDACTED]");
            let after = pos + prefix.len();
            let tail = &rest[after..];
            let skip = tail
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                .map(|c| c.len_utf8())
                .sum::<usize>();
            rest = &tail[skip..];
        } else {
            out.push_str(rest);
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_passthrough() {
        let p = prepare_observation_io("{\"a\":1}", IoPolicy::Structured);
        assert_eq!(p.text, "{\"a\":1}");
        assert!(p.complete);
    }

    #[test]
    fn preview_truncates_with_ellipsis() {
        let s = "a".repeat(100);
        let p = prepare_observation_io(&s, IoPolicy::Preview { max_bytes: 10 });
        assert!(p.text.ends_with('…'));
        assert!(!p.complete);
        assert!(std::str::from_utf8(p.text.as_bytes()).is_ok());
    }

    #[test]
    fn complete_preserves_long_ascii() {
        let mut s = "a".repeat(600);
        s.push_str(MARKER_TAIL_COMPLETE);
        let p = prepare_observation_io(&s, IoPolicy::Complete);
        assert!(p.complete);
        assert!(p.text.contains(MARKER_TAIL_COMPLETE));
        assert_eq!(p.text, s);
    }

    #[test]
    fn complete_preserves_multibyte() {
        let mut s = "é".repeat(400);
        s.push_str(MARKER_TAIL_COMPLETE);
        let p = prepare_observation_io(&s, IoPolicy::Complete);
        assert!(p.complete);
        assert!(p.text.contains(MARKER_TAIL_COMPLETE));
        assert_eq!(p.text, s);
    }

    #[test]
    fn redact_sk_lf_and_bearer() {
        let s = "key=sk-lf-abc123XYZ token=Bearer secrettoken rest";
        let out = redact_secrets(s);
        assert!(!out.contains("abc123XYZ"));
        assert!(!out.contains("secrettoken"));
        assert!(out.contains("sk-lf-[REDACTED]"));
        assert!(out.contains("Bearer [REDACTED]"));
    }

    #[test]
    fn complete_ceiling_marks_incomplete() {
        let s = "b".repeat(100);
        let p = prepare_complete_io(&s, 20);
        assert!(!p.complete);
        assert_eq!(p.io_bytes, 100);
        assert_eq!(p.text.len(), 20);
        assert!(std::str::from_utf8(p.text.as_bytes()).is_ok());
    }

    #[test]
    fn complete_zero_max_never_truncates() {
        let mut s = "c".repeat(2_000_000);
        s.push_str(MARKER_TAIL_COMPLETE);
        let p = prepare_complete_io(&s, 0);
        assert!(p.complete);
        assert_eq!(p.text, s);
        assert!(p.text.contains(MARKER_TAIL_COMPLETE));
    }

    #[test]
    fn complete_ceiling_utf8_safe() {
        let mut s = String::new();
        s.push_str(&"a".repeat(18));
        s.push('–'); // 3-byte en-dash
        s.push_str("tail");
        let p = prepare_complete_io(&s, 20);
        assert!(!p.complete);
        assert!(std::str::from_utf8(p.text.as_bytes()).is_ok());
        assert_eq!(p.text, "a".repeat(18));
    }

    #[test]
    fn format_chat_turns_notes_images() {
        let s = format_llm_chat_turns_for_observation([
            ("System", "rules", 0usize),
            ("User", "see this", 2usize),
        ]);
        assert!(s.contains("System: rules"));
        assert!(s.contains("User: see this"));
        assert!(s.contains("[2 image(s) attached]"));
    }
}

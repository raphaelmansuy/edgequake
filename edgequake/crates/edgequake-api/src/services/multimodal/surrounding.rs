//! LightRAG `multimodal_context.py` port — block-scoped surrounding with token budgets.

use edgequake_pipeline::default_recursive_separators;
use regex::Regex;
use std::sync::LazyLock;

const DEFAULT_SURROUNDING_MAX_TOKENS: usize = 2000;

/// Sidecar modality key aligned with LightRAG (`drawings` / `tables` / `equations`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurroundingKind {
    Drawings,
    Tables,
    Equations,
}

impl SurroundingKind {
    pub fn from_modality(modality: &str) -> Self {
        match modality {
            "table" => Self::Tables,
            "equation" => Self::Equations,
            _ => Self::Drawings,
        }
    }
}

/// Token counter for surrounding budgets (char mode for contract tests).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurroundingTokenCounter {
    /// 1 char ≈ 1 token (LightRAG unit-test mapping).
    Char,
    /// Production estimate (`edgequake_pipeline` word/CJK heuristic).
    Estimate,
}

impl SurroundingTokenCounter {
    pub fn from_env() -> Self {
        match std::env::var("EDGEQUAKE_MM_SURROUNDING_TOKENS")
            .ok()
            .as_deref()
            .map(str::trim)
        {
            Some("char") => Self::Char,
            _ => Self::Estimate,
        }
    }

    pub fn count(&self, text: &str) -> usize {
        match self {
            Self::Char => text.chars().count(),
            Self::Estimate => edgequake_pipeline::chunker::text_utils::estimate_tokens(text),
        }
    }
}

/// Resolve per-half token caps (LightRAG `SURROUNDING_*_MAX_TOKENS`, default 2000).
pub fn surrounding_leading_max_tokens() -> usize {
    env_usize("SURROUNDING_LEADING_MAX_TOKENS").unwrap_or(DEFAULT_SURROUNDING_MAX_TOKENS)
}

pub fn surrounding_trailing_max_tokens() -> usize {
    env_usize("SURROUNDING_TRAILING_MAX_TOKENS").unwrap_or(DEFAULT_SURROUNDING_MAX_TOKENS)
}

fn env_usize(name: &str) -> Option<usize> {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
}

/// Recursive separator cascade (`CHUNK_R_SEPARATORS` or LightRAG default).
pub fn load_chunk_separators() -> Vec<String> {
    if let Ok(raw) = std::env::var("CHUNK_R_SEPARATORS") {
        if let Ok(parsed) = serde_json::from_str::<Vec<String>>(&raw) {
            return parsed.into_iter().filter(|s| !s.is_empty()).collect();
        }
    }
    default_recursive_separators()
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect()
}

static MM_TAG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<drawing\b[^>]*/>|<table\b[^>]*>.*?</table>|<equation\b[^>]*>.*?</equation>")
        .expect("mm tag regex")
});

static CITE_REFID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\srefid\s*=\s*"[^"]*""#).expect("cite refid regex"));

static DRAWING_INTERNAL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)<drawing\b([^>]*)/>"#).expect("drawing regex"));

static TABLE_INTERNAL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<table\b([^>]*)>(.*?)</table>"#).expect("table internal regex")
});

static EQUATION_INTERNAL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<equation\b([^>]*)>(.*?)</equation>"#).expect("equation internal regex")
});

static INTERNAL_ATTRS: [&str; 4] = ["id", "path", "src", "refid"];

/// Locate target multimodal marker by `item_id` in block content.
pub fn find_target_span(
    kind: SurroundingKind,
    item_id: &str,
    block_content: &str,
) -> Option<(usize, usize)> {
    let modality = match kind {
        SurroundingKind::Drawings => "drawing",
        SurroundingKind::Tables => "table",
        SurroundingKind::Equations => "equation",
    };
    super::scan::span_for_item(block_content, item_id, modality)
}

fn atomize(text: &str) -> Vec<(AtomKind, String)> {
    let mut atoms = Vec::new();
    let mut pos = 0usize;
    for m in MM_TAG_RE.find_iter(text) {
        if m.start() > pos {
            atoms.push((AtomKind::Text, text[pos..m.start()].to_string()));
        }
        let tag = m.as_str();
        let kind = if tag.starts_with("<drawing") {
            AtomKind::Drawing
        } else if tag.starts_with("<table") {
            AtomKind::Table
        } else {
            AtomKind::Equation
        };
        atoms.push((kind, tag.to_string()));
        pos = m.end();
    }
    if pos < text.len() {
        atoms.push((AtomKind::Text, text[pos..].to_string()));
    }
    atoms
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AtomKind {
    Text,
    Drawing,
    Table,
    Equation,
}

pub fn remove_table_tags(text: &str) -> String {
    let mut result = text.to_string();
    loop {
        let lower = result.to_ascii_lowercase();
        let mut removed = false;
        if let Some(start) = lower.find("<table") {
            if let Some(rel_end) = lower[start..].find("</table>") {
                let end = start + rel_end + "</table>".len();
                result.replace_range(start..end, " ");
                removed = true;
            }
        }
        if let Some(start) = lower.find("<cite") {
            let attrs_end = lower[start..].find('>').map(|i| start + i + 1);
            if let Some(open_end) = attrs_end {
                let attrs = &result[start..open_end.min(result.len())];
                if attrs.to_ascii_lowercase().contains("type=\"table\"") {
                    if let Some(rel_end) = lower[open_end..].find("</cite>") {
                        let end = open_end + rel_end + "</cite>".len();
                        result.replace_range(start..end, " ");
                        removed = true;
                    }
                }
            }
        }
        if !removed {
            break;
        }
    }
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Strip parser-internal attrs before surrounding accumulation (LightRAG chunk_schema).
pub fn strip_internal_multimodal_markup(content: &str, keep_cite_tag: bool) -> String {
    if content.is_empty() {
        return content.to_string();
    }
    let mut cleaned = if keep_cite_tag {
        CITE_REFID_RE.replace_all(content, "").into_owned()
    } else {
        Regex::new(r#"(?s)<cite\b[^>]*>(.*?)</cite>"#)
            .expect("cite strip")
            .replace_all(content, "$1")
            .into_owned()
    };
    cleaned = DRAWING_INTERNAL_RE
        .replace_all(&cleaned, |caps: &regex::Captures| {
            let attrs = strip_internal_attrs(caps.get(1).map(|m| m.as_str()).unwrap_or(""));
            if attrs.trim().is_empty() {
                String::new()
            } else {
                format!("<drawing {attrs} />")
            }
        })
        .into_owned();
    cleaned = TABLE_INTERNAL_RE
        .replace_all(&cleaned, |caps: &regex::Captures| {
            let attrs = strip_internal_attrs(caps.get(1).map(|m| m.as_str()).unwrap_or(""));
            let body = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            format!("<table {attrs}>{body}</table>")
        })
        .into_owned();
    cleaned = EQUATION_INTERNAL_RE
        .replace_all(&cleaned, |caps: &regex::Captures| {
            let attrs = strip_internal_attrs(caps.get(1).map(|m| m.as_str()).unwrap_or(""));
            let body = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            format!("<equation {attrs}>{body}</equation>")
        })
        .into_owned();
    cleaned
}

fn strip_internal_attrs(attrs: &str) -> String {
    let mut s = attrs.trim().to_string();
    for name in INTERNAL_ATTRS {
        let re = Regex::new(&format!(r#"\b{name}\s*=\s*"[^"]*""#)).expect("attr strip regex");
        s = re.replace_all(&s, "").into_owned();
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn split_text_segment(text: &str, separators: &[String]) -> (Vec<String>, usize) {
    if text.is_empty() {
        return (vec![text.to_string()], separators.len());
    }
    for (idx, sep) in separators.iter().enumerate() {
        if sep.is_empty() {
            continue;
        }
        if text.contains(sep.as_str()) {
            let parts: Vec<&str> = text.split(sep).collect();
            let mut assembled = Vec::new();
            for (j, part) in parts.iter().enumerate() {
                if j < parts.len() - 1 {
                    assembled.push(format!("{part}{sep}"));
                } else if !part.is_empty() {
                    assembled.push((*part).to_string());
                }
            }
            if assembled.len() > 1 {
                return (assembled, idx);
            }
        }
    }
    (vec![text.to_string()], separators.len())
}

fn char_trim_leading(text: &str, max_tokens: usize, counter: SurroundingTokenCounter) -> String {
    if counter.count(text) <= max_tokens {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut lo = 0usize;
    let mut hi = chars.len();
    while lo < hi {
        let mid = (lo + hi) / 2;
        let suffix: String = chars[mid..].iter().collect();
        if counter.count(&suffix) <= max_tokens {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    chars[lo..].iter().collect()
}

pub fn char_trim_trailing(
    text: &str,
    max_tokens: usize,
    counter: SurroundingTokenCounter,
) -> String {
    if counter.count(text) <= max_tokens {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut lo = 0usize;
    let mut hi = chars.len();
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let prefix: String = chars[..mid].iter().collect();
        if counter.count(&prefix) <= max_tokens {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    chars[..lo].iter().collect()
}

static TABLE_TAG_OPEN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)^<table\b([^>]*)>(.*)</table>$"#).expect("table tag regex"));

fn detect_table_format(attrs: &str, body: &str) -> &'static str {
    let attrs_lower = attrs.to_ascii_lowercase();
    if attrs_lower.contains("format=\"json\"") || body.trim_start().starts_with('[') {
        "json"
    } else {
        "html"
    }
}

fn split_html_rows(body: &str) -> Vec<String> {
    let tr_re = Regex::new(r"(?si)<tr\b[^>]*>.*?</tr>").expect("tr regex");
    tr_re
        .find_iter(body)
        .map(|m| m.as_str().to_string())
        .collect()
}

fn parse_json_table_rows(body: &str) -> Option<Vec<serde_json::Value>> {
    serde_json::from_str(body.trim()).ok()
}

/// Row-aware table trim for leading context (keep tail rows closest to target).
pub fn row_trim_table_leading(
    tag_text: &str,
    max_tokens: usize,
    counter: SurroundingTokenCounter,
) -> Option<String> {
    let caps = TABLE_TAG_OPEN.captures(tag_text.trim())?;
    let attrs = caps.get(1)?.as_str();
    let body = caps.get(2)?.as_str();
    let fmt = detect_table_format(attrs, body);
    if fmt == "json" {
        let rows = parse_json_table_rows(body)?;
        for k in (1..rows.len()).rev() {
            let candidate = format!(
                "<table {attrs}>{}</table>",
                serde_json::to_string(&rows[rows.len() - k..]).unwrap_or_default()
            );
            if counter.count(&candidate) <= max_tokens {
                return Some(candidate);
            }
        }
    } else {
        let rows = split_html_rows(body);
        for k in (1..rows.len()).rev() {
            let inner = rows[rows.len() - k..].concat();
            let candidate = format!("<table {attrs}>{inner}</table>");
            if counter.count(&candidate) <= max_tokens {
                return Some(candidate);
            }
        }
    }
    None
}

/// Row-aware table trim for trailing context (keep head rows closest to target).
pub fn row_trim_table_trailing(
    tag_text: &str,
    max_tokens: usize,
    counter: SurroundingTokenCounter,
) -> Option<String> {
    let caps = TABLE_TAG_OPEN.captures(tag_text.trim())?;
    let attrs = caps.get(1)?.as_str();
    let body = caps.get(2)?.as_str();
    let fmt = detect_table_format(attrs, body);
    if fmt == "json" {
        let rows = parse_json_table_rows(body)?;
        for k in (1..rows.len()).rev() {
            let candidate = format!(
                "<table {attrs}>{}</table>",
                serde_json::to_string(&rows[..k]).unwrap_or_default()
            );
            if counter.count(&candidate) <= max_tokens {
                return Some(candidate);
            }
        }
    } else {
        let rows = split_html_rows(body);
        for k in (1..rows.len()).rev() {
            let inner = rows[..k].concat();
            let candidate = format!("<table {attrs}>{inner}</table>");
            if counter.count(&candidate) <= max_tokens {
                return Some(candidate);
            }
        }
    }
    None
}

fn accumulate_text_leading(
    text: &str,
    existing: &str,
    max_tokens: usize,
    separators: &[String],
    counter: SurroundingTokenCounter,
) -> Option<String> {
    let (segments, sep_idx) = split_text_segment(text, separators);
    if segments.is_empty() {
        return None;
    }
    let mut buf = String::new();
    for i in (0..segments.len()).rev() {
        let candidate = format!("{}{}", segments[i], buf);
        if counter.count(&(candidate.clone() + existing)) <= max_tokens {
            buf = candidate;
            continue;
        }
        if !buf.is_empty() {
            return Some(buf);
        }
        let weaker = separators.get(sep_idx + 1..).unwrap_or(&[]);
        if !weaker.is_empty() {
            return accumulate_text_leading(&segments[i], existing, max_tokens, weaker, counter);
        }
        let remaining = max_tokens.saturating_sub(counter.count(existing));
        if remaining == 0 {
            return None;
        }
        let trimmed = char_trim_leading(&segments[i], remaining, counter);
        return if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        };
    }
    if buf.is_empty() {
        None
    } else {
        Some(buf)
    }
}

fn accumulate_text_trailing(
    text: &str,
    existing: &str,
    max_tokens: usize,
    separators: &[String],
    counter: SurroundingTokenCounter,
) -> Option<String> {
    let (segments, sep_idx) = split_text_segment(text, separators);
    if segments.is_empty() {
        return None;
    }
    let mut buf = String::new();
    for seg in &segments {
        let candidate = format!("{buf}{seg}");
        if counter.count(&(existing.to_string() + &candidate)) <= max_tokens {
            buf = candidate;
            continue;
        }
        if !buf.is_empty() {
            return Some(buf);
        }
        let weaker = separators.get(sep_idx + 1..).unwrap_or(&[]);
        if !weaker.is_empty() {
            return accumulate_text_trailing(seg, existing, max_tokens, weaker, counter);
        }
        let remaining = max_tokens.saturating_sub(counter.count(existing));
        if remaining == 0 {
            return None;
        }
        let trimmed = char_trim_trailing(seg, remaining, counter);
        return if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        };
    }
    if buf.is_empty() {
        None
    } else {
        Some(buf)
    }
}

fn build_leading(
    source: &str,
    kind: SurroundingKind,
    max_tokens: usize,
    separators: &[String],
    counter: SurroundingTokenCounter,
) -> String {
    if source.is_empty() || max_tokens == 0 {
        return String::new();
    }
    let mut source = source.to_string();
    if kind == SurroundingKind::Tables {
        source = remove_table_tags(&source);
        if source.is_empty() {
            return String::new();
        }
    }
    source = strip_internal_multimodal_markup(&source, true);
    if source.is_empty() {
        return String::new();
    }
    let mut accumulated = String::new();
    let atoms = atomize(&source);
    for (atom_kind, atom_text) in atoms.into_iter().rev() {
        if atom_text.is_empty() {
            continue;
        }
        match atom_kind {
            AtomKind::Drawing | AtomKind::Equation => {
                let candidate = format!("{atom_text}{accumulated}");
                if counter.count(&candidate) <= max_tokens {
                    accumulated = candidate;
                } else {
                    break;
                }
            }
            AtomKind::Table => {
                let candidate = format!("{atom_text}{accumulated}");
                if counter.count(&candidate) <= max_tokens {
                    accumulated = candidate;
                } else {
                    let remaining = max_tokens.saturating_sub(counter.count(&accumulated));
                    if remaining > 0 {
                        if let Some(trimmed) =
                            row_trim_table_leading(&atom_text, remaining, counter)
                        {
                            accumulated = format!("{trimmed}{accumulated}");
                        }
                    }
                    break;
                }
            }
            AtomKind::Text => {
                let Some(addition) = accumulate_text_leading(
                    &atom_text,
                    &accumulated,
                    max_tokens,
                    separators,
                    counter,
                ) else {
                    break;
                };
                accumulated = format!("{addition}{accumulated}");
                if counter.count(&accumulated) >= max_tokens {
                    break;
                }
            }
        }
    }
    accumulated
}

fn build_trailing(
    source: &str,
    kind: SurroundingKind,
    max_tokens: usize,
    separators: &[String],
    counter: SurroundingTokenCounter,
) -> String {
    if source.is_empty() || max_tokens == 0 {
        return String::new();
    }
    let mut source = source.to_string();
    if kind == SurroundingKind::Tables {
        source = remove_table_tags(&source);
        if source.is_empty() {
            return String::new();
        }
    }
    source = strip_internal_multimodal_markup(&source, true);
    if source.is_empty() {
        return String::new();
    }
    let mut accumulated = String::new();
    for (atom_kind, atom_text) in atomize(&source) {
        if atom_text.is_empty() {
            continue;
        }
        match atom_kind {
            AtomKind::Drawing | AtomKind::Equation | AtomKind::Table => {
                let candidate = format!("{accumulated}{atom_text}");
                if counter.count(&candidate) <= max_tokens {
                    accumulated = candidate;
                } else if matches!(atom_kind, AtomKind::Table) {
                    let remaining = max_tokens.saturating_sub(counter.count(&accumulated));
                    if remaining > 0 {
                        if let Some(trimmed) =
                            row_trim_table_trailing(&atom_text, remaining, counter)
                        {
                            accumulated.push_str(&trimmed);
                        }
                    }
                    break;
                } else {
                    break;
                }
            }
            AtomKind::Text => {
                let Some(addition) = accumulate_text_trailing(
                    &atom_text,
                    &accumulated,
                    max_tokens,
                    separators,
                    counter,
                ) else {
                    break;
                };
                accumulated.push_str(&addition);
                if counter.count(&accumulated) >= max_tokens {
                    break;
                }
            }
        }
    }
    accumulated
}

/// Compute leading/trailing halves for one item span (LightRAG `build_surrounding`).
pub fn build_surrounding(
    kind: SurroundingKind,
    block_content: &str,
    span: (usize, usize),
    leading_max_tokens: usize,
    trailing_max_tokens: usize,
    separators: &[String],
    counter: SurroundingTokenCounter,
) -> super::context::SurroundingContext {
    let (start, end) = span;
    let leading = build_leading(
        &block_content[..start.min(block_content.len())],
        kind,
        leading_max_tokens,
        separators,
        counter,
    );
    let trailing = build_trailing(
        &block_content[end.min(block_content.len())..],
        kind,
        trailing_max_tokens,
        separators,
        counter,
    );
    super::context::SurroundingContext { leading, trailing }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_drawing_span_in_mixed_content() {
        let content = "leading text. <drawing id=\"im-abcd-0001\" format=\"png\" path=\"img.png\" src=\"img\" /> trailing text.";
        let span = find_target_span(SurroundingKind::Drawings, "im-abcd-0001", content).unwrap();
        let snippet = &content[span.0..span.1];
        assert!(snippet.starts_with("<drawing id=\"im-abcd-0001\""));
        assert!(snippet.ends_with("/>"));
    }

    #[test]
    fn table_surrounding_strips_sibling_tables() {
        let block = concat!(
            "<table id=\"tb-other\" format=\"json\">[[\"a\",\"b\"]]</table> ",
            "narrative text describing the report. ",
            "<table id=\"tb-target\" format=\"json\">[[\"x\",\"y\"]]</table>",
            " concluding remarks."
        );
        let span = find_target_span(SurroundingKind::Tables, "tb-target", block).unwrap();
        let surr = build_surrounding(
            SurroundingKind::Tables,
            block,
            span,
            2000,
            2000,
            &load_chunk_separators(),
            SurroundingTokenCounter::Char,
        );
        assert!(!surr.leading.contains("<table"));
        assert!(surr.leading.contains("narrative text"));
        assert!(surr.trailing.contains("concluding remarks"));
    }

    #[test]
    fn equation_surrounding_strips_drawing_internal_attrs() {
        let block = concat!(
            "<drawing id=\"im-prev\" path=\"a.png\" src=\"a\" caption=\"Fig 1\" />",
            " intro text. ",
            "<equation id=\"eq-1\" format=\"latex\">a+b=c</equation>",
            " conclusion text."
        );
        let span = find_target_span(SurroundingKind::Equations, "eq-1", block).unwrap();
        let surr = build_surrounding(
            SurroundingKind::Equations,
            block,
            span,
            2000,
            2000,
            &load_chunk_separators(),
            SurroundingTokenCounter::Char,
        );
        assert!(surr.leading.contains("<drawing caption=\"Fig 1\" />"));
        assert!(!surr.leading.contains("im-prev"));
    }
}

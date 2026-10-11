//! Verified citation rewrite (SPEC-142 / P0.5).
//!
//! # Laws
//!
//! - **LAW-142-1:** Locators come from the catalog (storage), never LLM tokens.
//! - **LAW-142-3:** Deterministic rewrite of valid `[N]` → markdown deeplinks.
//! - **LAW-142-5:** Unknown `[N]` is stripped.
//! - **LAW-142-6:** Acc gold callers skip this module.
//! - **LAW-142-11:** Uncatalogued prose page numerals are stripped (never promoted to chips).
//! - **LAW-142-12:** Multi-doc answers disambiguate chips with a short stem.
//! - **LAW-142-13:** Citation quality is observed via [`RewriteReport`] counts.
//!
//! # SOLID
//!
//! - **SRP:** Parse + rewrite + prose scrub only — no KV / retrieval.
//! - **DIP:** Takes [`CitationCatalog`], not storage traits.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;

use crate::context::RetrievedChunk;

/// One catalog entry keyed by prompt `[N]` / `reference_id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitationEntry {
    pub reference_id: usize,
    pub chunk_id: String,
    pub document_id: String,
    pub document_name: String,
    pub page_start: Option<u32>,
    pub page_end: Option<u32>,
}

impl CitationEntry {
    /// SPEC-033 / SPEC-135 deeplink: always open `page_start` when present.
    pub fn href(&self) -> String {
        build_document_page_url(
            &self.document_id,
            Some(self.chunk_id.as_str()),
            self.page_start,
        )
    }

    /// Visible link label.
    ///
    /// - Single-doc: `p.N` / `p.N–M` when page known; else short title.
    /// - Multi-doc (LAW-142-12): `stem p.N` so adjacent chips stay distinguishable.
    ///
    /// Full `document_name` goes in the markdown link title (hover / a11y).
    pub fn link_text(&self, multi_doc: bool) -> String {
        let page_part = match (self.page_start, self.page_end) {
            (Some(start), Some(end)) if end > start => Some(format!("p.{start}–{end}")),
            (Some(start), _) => Some(format!("p.{start}")),
            _ => None,
        };
        match page_part {
            Some(p) if multi_doc => {
                let stem = escape_markdown_link_text(&short_document_label(&self.document_name));
                format!("{stem} {p}")
            }
            Some(p) => p,
            None => escape_markdown_link_text(&short_document_label(&self.document_name)),
        }
    }

    /// Markdown link `[text](href "full document name")`.
    pub fn to_markdown_link(&self, multi_doc: bool) -> String {
        let title = escape_markdown_title(&self.document_name);
        format!(
            "[{}]({} \"{}\")",
            self.link_text(multi_doc),
            self.href(),
            title
        )
    }
}

/// Map of citation index → entry (prompt `[N]`).
#[derive(Debug, Clone, Default)]
pub struct CitationCatalog {
    by_id: HashMap<usize, CitationEntry>,
}

impl CitationCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, entry: CitationEntry) {
        self.by_id.insert(entry.reference_id, entry);
    }

    pub fn get(&self, id: usize) -> Option<&CitationEntry> {
        self.by_id.get(&id)
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn entries(&self) -> impl Iterator<Item = &CitationEntry> {
        self.by_id.values()
    }

    /// Union of stored page numbers from catalog entries (LAW-142-11 allow-list).
    pub fn allowed_pages(&self) -> HashSet<u32> {
        let mut pages = HashSet::new();
        for e in self.by_id.values() {
            match (e.page_start, e.page_end) {
                (Some(start), Some(end)) if end >= start => {
                    for p in start..=end {
                        pages.insert(p);
                    }
                }
                (Some(start), _) => {
                    pages.insert(start);
                }
                _ => {}
            }
        }
        pages
    }

    /// Build from retrieved chunks. `document_names` maps document_id → display title.
    ///
    /// Chunks without `citation_id` or without `document_id` are skipped.
    pub fn from_chunks(
        chunks: &[RetrievedChunk],
        document_names: &HashMap<String, String>,
    ) -> Self {
        let mut catalog = Self::new();
        for (i, chunk) in chunks.iter().enumerate() {
            let Some(ref_id) = chunk.citation_id.or(Some(i + 1)) else {
                continue;
            };
            let Some(doc_id) = chunk.document_id.as_ref().filter(|s| !s.is_empty()) else {
                tracing::debug!(
                    chunk_id = %chunk.id,
                    citation_id = ?chunk.citation_id,
                    "SPEC-142: skipping chunk without document_id (silent cite drop)"
                );
                continue;
            };
            let name = document_names
                .get(doc_id)
                .cloned()
                .or_else(|| chunk.document_name.clone())
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| short_doc_fallback(doc_id));
            catalog.insert(CitationEntry {
                reference_id: ref_id,
                chunk_id: chunk.id.clone(),
                document_id: doc_id.clone(),
                document_name: name,
                page_start: chunk.page_start,
                page_end: chunk.page_end,
            });
        }
        catalog
    }

    /// Build from API-shaped sources (chunk rows with optional reference_id).
    pub fn from_source_rows(rows: &[CitationSourceRow<'_>]) -> Self {
        let mut catalog = Self::new();
        for (i, row) in rows.iter().enumerate() {
            if row.source_type != "chunk" {
                continue;
            }
            let Some(doc_id) = row.document_id.filter(|s| !s.is_empty()) else {
                tracing::debug!(
                    chunk_id = %row.chunk_id,
                    reference_id = ?row.reference_id,
                    "SPEC-142: skipping source row without document_id (silent cite drop)"
                );
                continue;
            };
            let ref_id = row.reference_id.unwrap_or(i + 1);
            let name = row
                .document_name
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| short_doc_fallback(doc_id));
            catalog.insert(CitationEntry {
                reference_id: ref_id,
                chunk_id: row.chunk_id.to_string(),
                document_id: doc_id.to_string(),
                document_name: name,
                page_start: row.page_start,
                page_end: row.page_end,
            });
        }
        catalog
    }
}

/// Borrowed row used when building a catalog from HTTP `SourceReference` fields.
#[derive(Debug, Clone, Copy)]
pub struct CitationSourceRow<'a> {
    pub source_type: &'a str,
    pub chunk_id: &'a str,
    pub reference_id: Option<usize>,
    pub document_id: Option<&'a str>,
    pub document_name: Option<&'a str>,
    pub page_start: Option<u32>,
    pub page_end: Option<u32>,
}

/// Result of rewriting an answer (LAW-142-13 observability).
#[derive(Debug, Clone, Default)]
pub struct RewriteReport {
    pub text: String,
    pub rewritten_ids: Vec<usize>,
    pub stripped_ids: Vec<usize>,
    /// Count of uncatalogued prose page phrases removed (LAW-142-11).
    pub prose_pages_stripped: usize,
    /// Distinct `document_id`s among rewritten cites.
    pub unique_document_ids: usize,
    /// Crude share of claim sentences with no `/documents/` link (log-only; None if no claims).
    pub uncited_sentence_ratio: Option<f32>,
}

impl RewriteReport {
    /// `rewritten / (rewritten + stripped)` when any `[N]` was seen; else `None`.
    pub fn citation_validity(&self) -> Option<f32> {
        let ok = self.rewritten_ids.len();
        let bad = self.stripped_ids.len();
        let den = ok + bad;
        if den == 0 {
            None
        } else {
            Some(ok as f32 / den as f32)
        }
    }
}

/// Rewrite product answers: valid `[N]` → verified markdown links; unknown → strip;
/// then scrub uncatalogued prose page numerals (LAW-142-11).
///
/// Skips fenced code blocks (``` … ```). Does not rewrite Acc gold — callers must
/// skip this function when `is_gold_answer_extension` is true.
pub fn rewrite_verified_citations(answer: &str, catalog: &CitationCatalog) -> RewriteReport {
    if answer.is_empty() || catalog.is_empty() {
        return RewriteReport {
            text: answer.to_string(),
            ..Default::default()
        };
    }

    let multi_doc = will_cite_multiple_documents(answer, catalog);

    let mut rewritten_ids = Vec::new();
    let mut stripped_ids = Vec::new();
    let mut out = String::with_capacity(answer.len() + 64);
    let chars: Vec<char> = answer.chars().collect();
    let mut i = 0usize;
    let mut in_fence = false;

    while i < chars.len() {
        if starts_with_fence(&chars, i) {
            in_fence = !in_fence;
            out.push_str("```");
            i += 3;
            continue;
        }

        if !in_fence && chars[i] == '[' {
            if let Some((consumed, ids)) = parse_citation_bracket(&chars, i) {
                let mut first = true;
                for id in ids {
                    if let Some(entry) = catalog.get(id) {
                        if !first {
                            out.push(' ');
                        }
                        out.push_str(&entry.to_markdown_link(multi_doc));
                        rewritten_ids.push(id);
                        first = false;
                    } else {
                        stripped_ids.push(id);
                    }
                }
                i += consumed;
                continue;
            }
        }

        out.push(chars[i]);
        i += 1;
    }

    let allowed = catalog.allowed_pages();
    let (scrubbed, prose_pages_stripped) = scrub_uncatalogued_prose_pages(&out, &allowed);

    let unique_document_ids = {
        let mut docs = HashSet::new();
        for id in &rewritten_ids {
            if let Some(e) = catalog.get(*id) {
                docs.insert(e.document_id.as_str());
            }
        }
        docs.len()
    };

    let uncited_sentence_ratio = crude_uncited_sentence_ratio(&scrubbed);

    RewriteReport {
        text: scrubbed,
        rewritten_ids,
        stripped_ids,
        prose_pages_stripped,
        unique_document_ids,
        uncited_sentence_ratio,
    }
}

/// True when valid `[N]` cites in `answer` span more than one `document_id`.
fn will_cite_multiple_documents(answer: &str, catalog: &CitationCatalog) -> bool {
    let mut docs = HashSet::new();
    let chars: Vec<char> = answer.chars().collect();
    let mut i = 0usize;
    let mut in_fence = false;
    while i < chars.len() {
        if starts_with_fence(&chars, i) {
            in_fence = !in_fence;
            i += 3;
            continue;
        }
        if !in_fence && chars[i] == '[' {
            if let Some((consumed, ids)) = parse_citation_bracket(&chars, i) {
                for id in ids {
                    if let Some(e) = catalog.get(id) {
                        docs.insert(e.document_id.as_str());
                        if docs.len() > 1 {
                            return true;
                        }
                    }
                }
                i += consumed;
                continue;
            }
        }
        i += 1;
    }
    false
}

static PROSE_PAGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    // page(s) N, pages N–M / N-M, p.N, p. N, p.N–M
    Regex::new(
        r"(?i)(?:\bpages?\s+(\d+)\s*[–\-]\s*(\d+)\b|\bpages?\s+(\d+)\b|\bp\.\s*(\d+)\s*[–\-]\s*(\d+)\b|\bp\.\s*(\d+)\b)",
    )
    .expect("prose page regex")
});

/// Strip prose page phrases whose numerals are not in the catalog allow-list.
///
/// Skips fenced code and markdown links so `?page=N` hrefs are never touched.
/// Does **not** turn allowed prose pages into links (would invent attribution).
fn scrub_uncatalogued_prose_pages(text: &str, allowed: &HashSet<u32>) -> (String, usize) {
    if text.is_empty() {
        return (String::new(), 0);
    }

    let segments = split_protect_regions(text);
    let mut out = String::with_capacity(text.len());
    let mut stripped = 0usize;

    for seg in segments {
        if seg.protected {
            out.push_str(seg.text);
            continue;
        }
        let (piece, n) = scrub_prose_segment(seg.text, allowed);
        stripped += n;
        out.push_str(&piece);
    }

    (collapse_extra_spaces(&out), stripped)
}

struct TextSeg<'a> {
    text: &'a str,
    protected: bool,
}

/// Split into unprotected prose vs protected (fences + markdown links).
fn split_protect_regions(text: &str) -> Vec<TextSeg<'_>> {
    let chars: Vec<char> = text.chars().collect();
    let byte_of: Vec<usize> = {
        let mut v = Vec::with_capacity(chars.len() + 1);
        let mut off = 0usize;
        v.push(0);
        for c in &chars {
            off += c.len_utf8();
            v.push(off);
        }
        v
    };
    let mut segs = Vec::new();
    let mut i = 0usize;
    let mut prose_start = 0usize;

    while i < chars.len() {
        if starts_with_fence(&chars, i) {
            if prose_start < i {
                segs.push(TextSeg {
                    text: &text[byte_of[prose_start]..byte_of[i]],
                    protected: false,
                });
            }
            let start = i;
            i += 3;
            let mut closed = false;
            while i + 2 < chars.len() {
                if starts_with_fence(&chars, i) {
                    i += 3;
                    closed = true;
                    break;
                }
                i += 1;
            }
            if !closed {
                i = chars.len();
            }
            segs.push(TextSeg {
                text: &text[byte_of[start]..byte_of[i]],
                protected: true,
            });
            prose_start = i;
            continue;
        }

        if chars[i] == '[' {
            if let Some(end) = find_markdown_link_end(&chars, i) {
                if prose_start < i {
                    segs.push(TextSeg {
                        text: &text[byte_of[prose_start]..byte_of[i]],
                        protected: false,
                    });
                }
                segs.push(TextSeg {
                    text: &text[byte_of[i]..byte_of[end]],
                    protected: true,
                });
                i = end;
                prose_start = i;
                continue;
            }
        }

        i += 1;
    }
    if prose_start < chars.len() {
        segs.push(TextSeg {
            text: &text[byte_of[prose_start]..byte_of[chars.len()]],
            protected: false,
        });
    }
    segs
}

/// If `chars[i]` starts `[label](url…)` return char index after the closing `)`.
fn find_markdown_link_end(chars: &[char], i: usize) -> Option<usize> {
    if chars.get(i) != Some(&'[') {
        return None;
    }
    let mut j = i + 1;
    while j < chars.len() {
        if chars[j] == ']' {
            break;
        }
        // Unescaped `[` inside label → not our simple link
        if chars[j] == '[' {
            return None;
        }
        j += 1;
    }
    if j >= chars.len() || chars.get(j + 1) != Some(&'(') {
        return None;
    }
    j += 2; // past `](`
    let mut depth = 1i32;
    while j < chars.len() {
        match chars[j] {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(j + 1);
                }
            }
            _ => {}
        }
        j += 1;
    }
    None
}

fn scrub_prose_segment(segment: &str, allowed: &HashSet<u32>) -> (String, usize) {
    let mut stripped = 0usize;
    let mut out = String::with_capacity(segment.len());
    let mut last = 0usize;

    for caps in PROSE_PAGE_RE.captures_iter(segment) {
        let m = caps.get(0).expect("full match");
        let range_ok = if let (Some(a), Some(b)) = (caps.get(1), caps.get(2)) {
            parse_u32(a.as_str())
                .zip(parse_u32(b.as_str()))
                .is_some_and(|(s, e)| {
                    let (lo, hi) = if s <= e { (s, e) } else { (e, s) };
                    (lo..=hi).all(|p| allowed.contains(&p))
                })
        } else if let Some(a) = caps.get(3) {
            parse_u32(a.as_str()).is_some_and(|p| allowed.contains(&p))
        } else if let (Some(a), Some(b)) = (caps.get(4), caps.get(5)) {
            parse_u32(a.as_str())
                .zip(parse_u32(b.as_str()))
                .is_some_and(|(s, e)| {
                    let (lo, hi) = if s <= e { (s, e) } else { (e, s) };
                    (lo..=hi).all(|p| allowed.contains(&p))
                })
        } else if let Some(a) = caps.get(6) {
            parse_u32(a.as_str()).is_some_and(|p| allowed.contains(&p))
        } else {
            false
        };

        if range_ok {
            continue;
        }
        out.push_str(&segment[last..m.start()]);
        // Drop the phrase; leave a single space if neighbors need separation.
        stripped += 1;
        last = m.end();
    }
    out.push_str(&segment[last..]);
    (out, stripped)
}

fn parse_u32(s: &str) -> Option<u32> {
    s.parse().ok()
}

fn collapse_extra_spaces(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for c in s.chars() {
        if c == ' ' {
            if prev_space {
                continue;
            }
            prev_space = true;
            out.push(' ');
        } else {
            prev_space = false;
            if c == '\n' || c == '\r' || c == '\t' {
                // trim space before newline
                if out.ends_with(' ') {
                    out.pop();
                }
            }
            out.push(c);
        }
    }
    // Clean " ." / " ," left by phrase removal
    out = out.replace(" .", ".").replace(" ,", ",").replace(" ;", ";");
    out.trim().to_string()
}

/// Crude coverage: share of claim-like sentences with no `/documents/` markdown link.
///
/// Hedges / refusals are excluded from the denominator. Log-only — never rewrites.
fn crude_uncited_sentence_ratio(text: &str) -> Option<f32> {
    // Strip markdown links to inspect prose claims only for "has cite" — a sentence
    // is cited if it still contains `/documents/` (link was in that sentence).
    let mut claim = 0usize;
    let mut uncited = 0usize;
    for raw in text.split(['.', '!', '?']) {
        let s = raw.trim();
        if s.is_empty() {
            continue;
        }
        let lower = s.to_ascii_lowercase();
        if lower.starts_with("i don't")
            || lower.starts_with("i do not")
            || lower.starts_with("not answerable")
            || lower.starts_with("insufficient")
            || lower == "n/a"
        {
            continue;
        }
        // Skip pure link leftovers
        if s.chars().all(|c| c.is_whitespace() || c == '[' || c == ']') {
            continue;
        }
        claim += 1;
        if !s.contains("/documents/") {
            uncited += 1;
        }
    }
    if claim == 0 {
        None
    } else {
        Some(uncited as f32 / claim as f32)
    }
}

/// Mirror of WebUI `buildDocumentPageUrl` (SPEC-033 / SPEC-142 DRY).
pub fn build_document_page_url(doc_id: &str, chunk_id: Option<&str>, page: Option<u32>) -> String {
    let mut params: Vec<String> = Vec::new();
    if let Some(c) = chunk_id.map(str::trim).filter(|s| !s.is_empty()) {
        params.push(format!("chunk={c}"));
    }
    if let Some(p) = page.filter(|&p| p >= 1) {
        params.push(format!("page={p}"));
    }
    if params.is_empty() {
        format!("/documents/{doc_id}")
    } else {
        format!("/documents/{doc_id}?{}", params.join("&"))
    }
}

fn short_doc_fallback(doc_id: &str) -> String {
    let trimmed = doc_id.trim();
    if trimmed.len() <= 8 {
        trimmed.to_string()
    } else {
        format!("{}…", &trimmed[..8])
    }
}

/// Human-readable short label for no-page cites (basename, strip arxiv/version noise).
pub fn short_document_label(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name).trim();
    if base.is_empty() {
        return "document".to_string();
    }
    // Drop common arXiv-style suffix: `_2608.16157v1` before extension.
    let stem = if let Some((s, ext)) = base.rsplit_once('.') {
        let cleaned = strip_arxiv_version_suffix(s);
        if cleaned.is_empty() {
            base.to_string()
        } else if matches!(
            ext.to_ascii_lowercase().as_str(),
            "pdf" | "md" | "txt" | "docx" | "html"
        ) {
            cleaned
        } else {
            format!("{cleaned}.{ext}")
        }
    } else {
        strip_arxiv_version_suffix(base)
    };
    if stem.is_empty() {
        base.to_string()
    } else {
        stem
    }
}

fn strip_arxiv_version_suffix(stem: &str) -> String {
    regex_lite_arxiv_suffix(stem).unwrap_or_else(|| stem.to_string())
}

/// Strip trailing `_2608.16157v1` / `.2608.16157v1` without pulling in the `regex` crate path.
fn regex_lite_arxiv_suffix(stem: &str) -> Option<String> {
    let bytes = stem.as_bytes();
    let n = bytes.len();
    if n < 10 {
        return None;
    }
    let mut i = n;
    while i > 0 && bytes[i - 1].is_ascii_digit() {
        i -= 1;
    }
    if i == n || i == 0 || bytes[i - 1] != b'v' {
        return None;
    }
    let v_pos = i - 1;
    let mut j = v_pos;
    while j > 0 && bytes[j - 1].is_ascii_digit() {
        j -= 1;
    }
    if j == v_pos || j == 0 || bytes[j - 1] != b'.' {
        return None;
    }
    let dot = j - 1;
    let mut k = dot;
    while k > 0 && bytes[k - 1].is_ascii_digit() {
        k -= 1;
    }
    if k == dot || (dot - k) < 4 {
        return None;
    }
    if k == 0 {
        return None;
    }
    let sep = bytes[k - 1];
    if sep != b'_' && sep != b'.' {
        return None;
    }
    let cut = k - 1;
    if cut == 0 {
        return None;
    }
    Some(stem[..cut].to_string())
}

/// Escape `]` and `[` in link labels so markdown stays well-formed.
fn escape_markdown_link_text(name: &str) -> String {
    name.replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

/// Escape quotes in markdown link titles.
fn escape_markdown_title(name: &str) -> String {
    name.replace('\\', "\\\\").replace('"', "\\\"")
}

fn starts_with_fence(chars: &[char], i: usize) -> bool {
    i + 2 < chars.len() && chars[i] == '`' && chars[i + 1] == '`' && chars[i + 2] == '`'
}

/// Parse `[1]`, `[1][2]`, or `[1, 2]` / `[1,2]` at `i`. Returns (chars_consumed, ids).
fn parse_citation_bracket(chars: &[char], i: usize) -> Option<(usize, Vec<usize>)> {
    if chars.get(i) != Some(&'[') {
        return None;
    }
    let mut j = i + 1;
    let mut ids = Vec::new();

    let (end, first_ids) = parse_bracket_body(chars, j)?;
    ids.extend(first_ids);
    j = end;

    while j < chars.len() && chars[j] == '[' {
        let Some((end2, more)) = parse_bracket_body(chars, j + 1) else {
            break;
        };
        if more.is_empty() {
            break;
        }
        ids.extend(more);
        j = end2;
    }

    if ids.is_empty() {
        return None;
    }
    Some((j - i, ids))
}

/// Parse body starting at first char after `[`. Returns index after `]` and ids.
fn parse_bracket_body(chars: &[char], start: usize) -> Option<(usize, Vec<usize>)> {
    let mut j = start;
    let mut ids = Vec::new();
    let mut current = String::new();

    while j < chars.len() {
        let c = chars[j];
        if c == ']' {
            push_id_token(&mut ids, &current)?;
            return Some((j + 1, ids));
        }
        if c.is_ascii_digit() {
            current.push(c);
            j += 1;
            continue;
        }
        if c == ',' || c == ' ' || c == ';' {
            push_id_token(&mut ids, &current)?;
            current.clear();
            j += 1;
            continue;
        }
        return None;
    }
    None
}

fn push_id_token(ids: &mut Vec<usize>, token: &str) -> Option<()> {
    let t = token.trim();
    if t.is_empty() {
        return Some(());
    }
    let id: usize = t.parse().ok()?;
    if id == 0 {
        return None;
    }
    ids.push(id);
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog_fixture() -> CitationCatalog {
        let mut c = CitationCatalog::new();
        c.insert(CitationEntry {
            reference_id: 1,
            chunk_id: "chunk-a".into(),
            document_id: "doc-uuid".into(),
            document_name: "Fixture.pdf".into(),
            page_start: Some(4),
            page_end: Some(4),
        });
        c.insert(CitationEntry {
            reference_id: 2,
            chunk_id: "chunk-b".into(),
            document_id: "doc-uuid".into(),
            document_name: "Fixture.pdf".into(),
            page_start: Some(3),
            page_end: Some(4),
        });
        c
    }

    #[test]
    fn u142_01_rewrites_valid_strips_unknown_href_not_999() {
        let catalog = catalog_fixture();
        let raw = "The value is 42 [1]. See also [99] and page 999.";
        let report = rewrite_verified_citations(raw, &catalog);
        assert!(
            report
                .text
                .contains("[p.4](/documents/doc-uuid?chunk=chunk-a&page=4 \"Fixture.pdf\")"),
            "got: {}",
            report.text
        );
        assert!(!report.text.contains("[99]"), "got: {}", report.text);
        assert!(!report.text.contains("page=999"), "got: {}", report.text);
        assert!(
            !report.text.contains("page 999"),
            "LAW-142-11 prose scrub: {}",
            report.text
        );
        assert!(report.prose_pages_stripped >= 1);
        assert_eq!(report.rewritten_ids, vec![1]);
        assert_eq!(report.stripped_ids, vec![99]);
        assert_eq!(report.unique_document_ids, 1);
    }

    #[test]
    fn u142_01_keeps_catalogued_prose_page() {
        let catalog = catalog_fixture();
        let report = rewrite_verified_citations("See page 4 for details [1].", &catalog);
        assert!(
            report.text.contains("page 4"),
            "catalogued prose stays: {}",
            report.text
        );
        assert_eq!(report.prose_pages_stripped, 0);
        assert!(
            !report.text.contains("](page 4"),
            "must not invent link from prose"
        );
    }

    #[test]
    fn u142_01_compound_and_span_badge() {
        let catalog = catalog_fixture();
        let report = rewrite_verified_citations("Facts [1][2] and [1, 2].", &catalog);
        assert!(report.text.contains("[p.4]("), "got: {}", report.text);
        assert!(report.text.contains("[p.3–4]("), "got: {}", report.text);
        assert!(
            report.text.contains("page=3"),
            "span href uses page_start: {}",
            report.text
        );
        assert!(
            report.text.contains("&page=4 \""),
            "ref 1 href page=4: {}",
            report.text
        );
    }

    #[test]
    fn u142_12_multi_doc_chips_include_stem() {
        let mut c = CitationCatalog::new();
        c.insert(CitationEntry {
            reference_id: 1,
            chunk_id: "c1".into(),
            document_id: "doc-a".into(),
            document_name: "Alpha.pdf".into(),
            page_start: Some(5),
            page_end: Some(5),
        });
        c.insert(CitationEntry {
            reference_id: 2,
            chunk_id: "c2".into(),
            document_id: "doc-b".into(),
            document_name: "Beta_Report.pdf".into(),
            page_start: Some(6),
            page_end: Some(6),
        });
        let report = rewrite_verified_citations("A [1] and B [2].", &c);
        assert!(report.text.contains("[Alpha p.5]("), "got: {}", report.text);
        assert!(
            report.text.contains("[Beta_Report p.6]("),
            "got: {}",
            report.text
        );
        assert!(report.text.contains("\"Alpha.pdf\""));
        assert!(report.text.contains("\"Beta_Report.pdf\""));
        assert_eq!(report.unique_document_ids, 2);
    }

    #[test]
    fn u142_02_fenced_code_not_rewritten() {
        let catalog = catalog_fixture();
        let raw = "Before [1]\n```\ncode [1]\npage 999\n```\nAfter [1]";
        let report = rewrite_verified_citations(raw, &catalog);
        assert!(
            report.text.contains("code [1]"),
            "fence preserved: {}",
            report.text
        );
        assert!(
            report.text.contains("page 999"),
            "fence prose not scrubbed: {}",
            report.text
        );
        assert!(
            report.text.matches("[p.4](").count() >= 2,
            "outside fence rewritten: {}",
            report.text
        );
        assert!(
            report.text.contains("\"Fixture.pdf\""),
            "title carries full name: {}",
            report.text
        );
    }

    #[test]
    fn build_url_mirrors_webui_rules() {
        assert_eq!(build_document_page_url("d1", None, None), "/documents/d1");
        assert_eq!(
            build_document_page_url("d1", Some("c1"), Some(3)),
            "/documents/d1?chunk=c1&page=3"
        );
        assert_eq!(
            build_document_page_url("d1", Some("c1"), Some(0)),
            "/documents/d1?chunk=c1"
        );
        assert_eq!(
            build_document_page_url("d1", Some(""), Some(3)),
            "/documents/d1?page=3"
        );
    }

    #[test]
    fn escapes_brackets_in_document_name() {
        let mut c = CitationCatalog::new();
        c.insert(CitationEntry {
            reference_id: 1,
            chunk_id: "c".into(),
            document_id: "d".into(),
            document_name: "A] B".into(),
            page_start: None,
            page_end: None,
        });
        let report = rewrite_verified_citations("x [1]", &c);
        assert!(
            report
                .text
                .contains(r#"[A\] B](/documents/d?chunk=c "A] B")"#)
                || report
                    .text
                    .contains("[A\\] B](/documents/d?chunk=c \"A] B\")"),
            "got: {}",
            report.text
        );
    }

    #[test]
    fn short_document_label_strips_arxiv_noise() {
        assert_eq!(
            short_document_label("free_token_2608.16157v1.pdf"),
            "free_token"
        );
        assert_eq!(short_document_label("notes.md"), "notes");
        assert_eq!(short_document_label("Report.pdf"), "Report");
    }

    #[test]
    fn citation_parser_keeps_valid_prefix_before_non_citation_brackets() {
        let raw: Vec<char> = "[1][note]".chars().collect();
        assert_eq!(parse_citation_bracket(&raw, 0), Some((3, vec![1])));
        let raw: Vec<char> = "[1][2][note]".chars().collect();
        assert_eq!(parse_citation_bracket(&raw, 0), Some((6, vec![1, 2])));
        let report = rewrite_verified_citations("Fact [1][note].", &catalog_fixture());
        assert_eq!(report.rewritten_ids, vec![1]);
        assert!(report.text.contains("[p.4]("));
        assert!(report.text.contains("[note]"));
        let raw: Vec<char> = "[note][1]".chars().collect();
        assert_eq!(parse_citation_bracket(&raw, 0), None);
    }

    #[test]
    fn from_chunks_uses_document_name_map() {
        let chunk = RetrievedChunk::new("c1", "hello", 0.9)
            .with_document_id("doc-1")
            .with_citation_id(1)
            .with_page(4);
        let mut names = HashMap::new();
        names.insert("doc-1".into(), "Report.pdf".into());
        let catalog = CitationCatalog::from_chunks(&[chunk], &names);
        let e = catalog.get(1).unwrap();
        assert_eq!(e.document_name, "Report.pdf");
        assert_eq!(e.page_start, Some(4));
        assert_eq!(e.link_text(false), "p.4");
    }
}

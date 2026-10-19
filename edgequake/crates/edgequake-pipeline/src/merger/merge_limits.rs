//! SPEC-047 P7b / P7d — LightRAG merge concurrency + SOURCE_IDS KEEP (SSOT).
//!
//! # Law (LightRAG)
//!
//! - `graph_max_async = llm_model_max_async * 2` (default 4 → 8)
//! - `MAX_SOURCE_IDS_PER_{ENTITY,RELATION}` default **200**
//! - `SOURCE_IDS_LIMIT_METHOD=KEEP` (default): once existing chunk ids are
//!   saturated, **skip description updates** for brand-new chunks (keep oldest);
//!   when truncating, **round-robin across documents** (021 L-A4) so minority
//!   parents are not wiped by a majority doc flood
//! - `FIFO`: always merge; truncate to newest `limit` ids
//!
//! # SOLID
//!
//! - **S**: Pure limit / concurrency / diversity policy — no graph I/O
//! - **O**: Tunable via env / [`MergerConfig`](super::MergerConfig)
//! - **D**: Callers decide how to apply skip vs truncate; doc-key via lineage SSOT

use std::collections::{HashMap, HashSet};

use serde_json::Value;

/// LightRAG `DEFAULT_MAX_SOURCE_IDS_PER_ENTITY` / `…_RELATION`.
pub const DEFAULT_MAX_SOURCE_IDS: usize = 200;

/// LightRAG default `llm_model_max_async` (4) × 2 → graph merge concurrency.
pub const DEFAULT_MERGE_MAX_ASYNC: usize = 8;

/// Strategy for capping chunk source ids on entity/relation nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SourceIdsLimitMethod {
    /// Keep oldest ids; skip description merge when saturated + only new chunks.
    #[default]
    Keep,
    /// Keep newest ids; always allow description merge (truncate after).
    Fifo,
}

impl SourceIdsLimitMethod {
    /// Parse LightRAG / EdgeQuake env values (`KEEP` | `FIFO`).
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_uppercase().as_str() {
            "FIFO" => Self::Fifo,
            _ => Self::Keep, // KEEP, empty, unknown → KEEP (LightRAG fallback)
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Keep => "KEEP",
            Self::Fifo => "FIFO",
        }
    }
}

/// Env: `EDGEQUAKE_SOURCE_IDS_LIMIT_METHOD` (alias `SOURCE_IDS_LIMIT_METHOD`).
pub fn source_ids_limit_method_from_env() -> SourceIdsLimitMethod {
    let raw = std::env::var("EDGEQUAKE_SOURCE_IDS_LIMIT_METHOD")
        .or_else(|_| std::env::var("SOURCE_IDS_LIMIT_METHOD"))
        .unwrap_or_default();
    SourceIdsLimitMethod::parse(&raw)
}

/// Env: `EDGEQUAKE_MAX_SOURCE_IDS_PER_ENTITY` (alias `MAX_SOURCE_IDS_PER_ENTITY`).
pub fn max_source_ids_per_entity_from_env() -> usize {
    parse_max_source_ids(
        &std::env::var("EDGEQUAKE_MAX_SOURCE_IDS_PER_ENTITY")
            .or_else(|_| std::env::var("MAX_SOURCE_IDS_PER_ENTITY"))
            .unwrap_or_default(),
    )
}

/// Env: `EDGEQUAKE_MAX_SOURCE_IDS_PER_RELATION` (alias `MAX_SOURCE_IDS_PER_RELATION`).
pub fn max_source_ids_per_relation_from_env() -> usize {
    parse_max_source_ids(
        &std::env::var("EDGEQUAKE_MAX_SOURCE_IDS_PER_RELATION")
            .or_else(|_| std::env::var("MAX_SOURCE_IDS_PER_RELATION"))
            .unwrap_or_default(),
    )
}

/// Pure parser for max source-id caps (testable).
pub fn parse_max_source_ids(raw: &str) -> usize {
    raw.trim()
        .parse::<usize>()
        .ok()
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_MAX_SOURCE_IDS)
}

/// Local-provider ceiling for parallel unique entity/rel merges.
pub const LOCAL_MERGE_MAX_ASYNC: usize = 2;

/// Env: `EDGEQUAKE_MERGE_MAX_ASYNC`.
///
/// Fallback: `EDGEQUAKE_LLM_MAX_ASYNC` / `MAX_ASYNC` × 2, else
/// `EDGEQUAKE_EMBED_MAX_ASYNC` × 2, else [`DEFAULT_MERGE_MAX_ASYNC`].
///
/// This is the **requested** fan-out (provider-agnostic). The local-provider
/// cap is applied where the LLM that serves the merge is known — see
/// `MergerConfig::for_provider` — never from the process default provider.
pub fn merge_max_async_requested_from_env() -> usize {
    if let Ok(raw) = std::env::var("EDGEQUAKE_MERGE_MAX_ASYNC") {
        if let Some(n) = parse_merge_max_async(&raw) {
            return n;
        }
    }
    let llm_async = std::env::var("EDGEQUAKE_LLM_MAX_ASYNC")
        .or_else(|_| std::env::var("MAX_ASYNC"))
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|&n| n > 0);
    if let Some(n) = llm_async {
        return (n.saturating_mul(2)).clamp(1, 64);
    }
    let embed = std::env::var("EDGEQUAKE_EMBED_MAX_ASYNC")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|&n| n > 0);
    if let Some(n) = embed {
        return (n.saturating_mul(2)).clamp(1, 64);
    }
    DEFAULT_MERGE_MAX_ASYNC
}

/// Cap merge fan-out for capacity-bound local providers (SSOT helper).
pub fn apply_local_merge_async_clamp(requested: usize, provider_name: &str) -> usize {
    let bounded = requested.clamp(1, 64);
    let effective =
        crate::pipeline::cap_for_local_provider(provider_name, bounded, LOCAL_MERGE_MAX_ASYNC);
    if effective < bounded {
        tracing::info!(
            provider = provider_name,
            requested = bounded,
            effective,
            "Local merge concurrency clamped (set EDGEQUAKE_ALLOW_LOCAL_HIGH_CONCURRENCY=1 to override)"
        );
    }
    effective
}

/// Pure parser for `EDGEQUAKE_MERGE_MAX_ASYNC` (clamped 1..=64).
pub fn parse_merge_max_async(raw: &str) -> Option<usize> {
    raw.trim()
        .parse::<usize>()
        .ok()
        .filter(|&n| n > 0)
        .map(|n| n.min(64))
}

/// Default number of vector slices the durable fleet step writes at once.
///
/// WHY 4: an HNSW insert is CPU and buffer bound, not round-trip bound. Measured
/// on 1024-d halfvec: 1 session = 2.7 ms/row, 4 sessions = 1.2 ms/row effective
/// (2.2x). More sessions only fight for the same index pages and pool slots.
pub const DEFAULT_FLEET_VECTOR_CONCURRENCY: usize = 4;

/// Env: `EDGEQUAKE_FLEET_VECTOR_CONCURRENCY` (clamped 1..=16; `1` = sequential).
pub fn fleet_vector_concurrency_from_env() -> usize {
    std::env::var("EDGEQUAKE_FLEET_VECTOR_CONCURRENCY")
        .ok()
        .and_then(|raw| parse_fleet_vector_concurrency(&raw))
        .unwrap_or(DEFAULT_FLEET_VECTOR_CONCURRENCY)
}

/// Pure parser for `EDGEQUAKE_FLEET_VECTOR_CONCURRENCY` (clamped 1..=16).
pub fn parse_fleet_vector_concurrency(raw: &str) -> Option<usize> {
    raw.trim()
        .parse::<usize>()
        .ok()
        .filter(|&n| n > 0)
        .map(|n| n.min(16))
}

/// Merge two source-id sequences (order-preserving, deduped) — LightRAG `merge_source_ids`.
pub fn merge_source_ids(existing: &[String], incoming: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(existing.len() + incoming.len());
    let mut seen = HashSet::new();
    for id in existing.iter().chain(incoming.iter()) {
        if id.is_empty() {
            continue;
        }
        if seen.insert(id.as_str()) {
            out.push(id.clone());
        }
    }
    out
}

/// Doc key for diversity: `{doc}` from `{doc}-chunk-N`, else the chunk id itself.
fn diversity_doc_key(chunk_id: &str) -> String {
    super::lineage::document_id_from_chunk_id(chunk_id).unwrap_or_else(|| chunk_id.to_string())
}

/// KEEP truncation with **document diversity** (SPEC-047 / 021 L-A4 · L6).
///
/// Round-robin across documents (oldest-first within each doc) so a saturated
/// majority doc cannot wipe minority parents when capping.
///
/// Single-doc / undocumentable ids behave like classic KEEP (head).
pub fn truncate_keep_doc_diverse(source_ids: &[String], limit: usize) -> Vec<String> {
    if limit == 0 {
        return Vec::new();
    }
    if source_ids.len() <= limit {
        return source_ids.to_vec();
    }

    // Preserve first-seen doc order; within each doc keep original (oldest) order.
    let mut group_order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, Vec<&String>> = HashMap::new();
    for id in source_ids {
        if id.is_empty() {
            continue;
        }
        let key = diversity_doc_key(id);
        if !groups.contains_key(&key) {
            group_order.push(key.clone());
        }
        groups.entry(key).or_default().push(id);
    }

    if group_order.len() <= 1 {
        // Classic KEEP head — no diversity to preserve.
        return source_ids.iter().take(limit).cloned().collect();
    }

    let mut cursors: HashMap<&str, usize> =
        group_order.iter().map(|k| (k.as_str(), 0usize)).collect();
    let mut out = Vec::with_capacity(limit);
    let mut progressed = true;
    while out.len() < limit && progressed {
        progressed = false;
        for key in &group_order {
            if out.len() >= limit {
                break;
            }
            let Some(ids) = groups.get(key) else {
                continue;
            };
            let cursor = cursors
                .get_mut(key.as_str())
                .expect("cursor for known group");
            if *cursor < ids.len() {
                out.push(ids[*cursor].clone());
                *cursor += 1;
                progressed = true;
            }
        }
    }
    out
}

/// Apply KEEP (doc-diverse head) or FIFO (tail) truncation — LightRAG + 021 L-A4.
pub fn apply_source_ids_limit(
    source_ids: &[String],
    limit: usize,
    method: SourceIdsLimitMethod,
) -> Vec<String> {
    if limit == 0 {
        return Vec::new();
    }
    if source_ids.len() <= limit {
        return source_ids.to_vec();
    }
    match method {
        SourceIdsLimitMethod::Fifo => source_ids[source_ids.len() - limit..].to_vec(),
        SourceIdsLimitMethod::Keep => truncate_keep_doc_diverse(source_ids, limit),
    }
}

/// LightRAG KEEP skip: saturated existing ids + no incoming id already known → skip merge.
///
/// Soft-resume of an already-linked chunk still updates (incoming ∩ existing ≠ ∅).
pub fn should_skip_description_update_keep(
    existing_source_ids: &[String],
    incoming_source_ids: &[String],
    max_source_ids: usize,
    method: SourceIdsLimitMethod,
) -> bool {
    if method != SourceIdsLimitMethod::Keep {
        return false;
    }
    if max_source_ids == 0 {
        return true;
    }
    if existing_source_ids.len() < max_source_ids {
        return false;
    }
    let existing: HashSet<&str> = existing_source_ids.iter().map(String::as_str).collect();
    !incoming_source_ids
        .iter()
        .any(|id| !id.is_empty() && existing.contains(id.as_str()))
}

/// Read `source_chunk_ids` from graph properties (array), with singular fallback.
pub fn source_chunk_ids_from_properties(props: &HashMap<String, Value>) -> Vec<String> {
    if let Some(arr) = props
        .get("source_chunk_ids")
        .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok())
    {
        return arr.into_iter().filter(|s| !s.is_empty()).collect();
    }
    if let Some(arr) = props
        .get("source_ids")
        .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok())
    {
        return arr.into_iter().filter(|s| !s.is_empty()).collect();
    }
    props
        .get("source_chunk_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| vec![s.to_string()])
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fleet_vector_concurrency_parses_and_clamps() {
        assert_eq!(parse_fleet_vector_concurrency("4"), Some(4));
        assert_eq!(parse_fleet_vector_concurrency(" 1 "), Some(1));
        assert_eq!(parse_fleet_vector_concurrency("99"), Some(16));
        assert_eq!(parse_fleet_vector_concurrency("0"), None);
        assert_eq!(parse_fleet_vector_concurrency("many"), None);
        assert_eq!(DEFAULT_FLEET_VECTOR_CONCURRENCY, 4);
    }

    #[test]
    fn keep_truncates_head() {
        let ids: Vec<_> = (0..5).map(|i| format!("c{i}")).collect();
        assert_eq!(
            apply_source_ids_limit(&ids, 3, SourceIdsLimitMethod::Keep),
            vec!["c0", "c1", "c2"]
        );
    }

    #[test]
    fn keep_single_doc_is_classic_head() {
        let ids: Vec<_> = (0..5).map(|i| format!("doc-a-chunk-{i}")).collect();
        assert_eq!(
            apply_source_ids_limit(&ids, 3, SourceIdsLimitMethod::Keep),
            vec![
                "doc-a-chunk-0".to_string(),
                "doc-a-chunk-1".to_string(),
                "doc-a-chunk-2".to_string()
            ]
        );
    }

    #[test]
    fn keep_doc_diverse_retains_minority_doc() {
        // Naive KEEP head would take only doc-a (first 3) and wipe doc-b.
        let mut ids = Vec::new();
        for i in 0..5 {
            ids.push(format!("doc-a-chunk-{i}"));
        }
        ids.push("doc-b-chunk-0".into());
        ids.push("doc-b-chunk-1".into());
        let capped = apply_source_ids_limit(&ids, 3, SourceIdsLimitMethod::Keep);
        assert_eq!(capped.len(), 3);
        let docs: HashSet<_> = capped
            .iter()
            .filter_map(|id| super::super::lineage::document_id_from_chunk_id(id))
            .collect();
        assert!(docs.contains("doc-a"), "majority retained");
        assert!(
            docs.contains("doc-b"),
            "021 L-A4: minority doc must survive cap; got {capped:?}"
        );
    }

    #[test]
    fn fifo_truncates_tail() {
        let ids: Vec<_> = (0..5).map(|i| format!("c{i}")).collect();
        assert_eq!(
            apply_source_ids_limit(&ids, 3, SourceIdsLimitMethod::Fifo),
            vec!["c2", "c3", "c4"]
        );
    }

    #[test]
    fn keep_skips_when_saturated_and_only_new_chunks() {
        let existing: Vec<_> = (0..200).map(|i| format!("old{i}")).collect();
        assert!(should_skip_description_update_keep(
            &existing,
            &["brand-new".into()],
            200,
            SourceIdsLimitMethod::Keep,
        ));
    }

    #[test]
    fn keep_allows_soft_resume_of_known_chunk() {
        let existing: Vec<_> = (0..200).map(|i| format!("old{i}")).collect();
        assert!(!should_skip_description_update_keep(
            &existing,
            &["old0".into()],
            200,
            SourceIdsLimitMethod::Keep,
        ));
    }

    #[test]
    fn fifo_never_skips_via_keep_gate() {
        let existing: Vec<_> = (0..200).map(|i| format!("old{i}")).collect();
        assert!(!should_skip_description_update_keep(
            &existing,
            &["brand-new".into()],
            200,
            SourceIdsLimitMethod::Fifo,
        ));
    }

    #[test]
    fn merge_source_ids_dedupes_order() {
        assert_eq!(
            merge_source_ids(&["a".into(), "b".into()], &["b".into(), "c".into()]),
            vec!["a", "b", "c"]
        );
    }

    #[test]
    fn parse_method_defaults_keep() {
        assert_eq!(SourceIdsLimitMethod::parse(""), SourceIdsLimitMethod::Keep);
        assert_eq!(
            SourceIdsLimitMethod::parse("fifo"),
            SourceIdsLimitMethod::Fifo
        );
    }
}

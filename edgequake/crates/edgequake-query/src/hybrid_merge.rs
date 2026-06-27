//! LightRAG-style Hybrid merge (SPEC-024 Phase 2 / FEAT0104).
//!
//! Combines Local, Global, and Naive retrieval arms with:
//! - **Round-robin interleave** (default): KG-derived arms first at each slot
//!   (local → global → naive), deduplicating by chunk ID — matches LightRAG.
//! - **RRF** (optional): set `EDGEQUAKE_HYBRID_FUSION=rrf` for reciprocal rank fusion.
//!
//! Entities and relationships are unioned across local+global (scores are not
//! cross-arm comparable). Chunks are truncated to `max_chunks`.

use std::collections::{HashMap, HashSet};

use crate::context::{QueryContext, RetrievedChunk, RetrievedRelationship};
use crate::fusion;

/// How Hybrid mode merges chunk lists from the three retrieval arms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridFusionMode {
    /// LightRAG default: round-robin interleave with KG-first priority.
    RoundRobin,
    /// RRF across ranked lists (equal weights).
    Rrf,
}

/// Read Hybrid chunk fusion mode (default: round-robin / LightRAG).
pub fn hybrid_fusion_mode_from_env() -> HybridFusionMode {
    match std::env::var("EDGEQUAKE_HYBRID_FUSION")
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "rrf" => HybridFusionMode::Rrf,
        _ => HybridFusionMode::RoundRobin,
    }
}

/// Operator-visible label for health / dashboards.
pub fn hybrid_fusion_mode_label(mode: HybridFusionMode) -> &'static str {
    match mode {
        HybridFusionMode::RoundRobin => "round_robin",
        HybridFusionMode::Rrf => "rrf",
    }
}

/// Merge three retrieval contexts into one Hybrid context.
pub fn merge_hybrid_contexts(
    local: QueryContext,
    global: QueryContext,
    naive: QueryContext,
    max_chunks: usize,
) -> QueryContext {
    let mut merged = QueryContext::new();

    let chunks = merge_hybrid_chunks(
        &local.chunks,
        &global.chunks,
        &naive.chunks,
        max_chunks,
        hybrid_fusion_mode_from_env(),
    );
    for chunk in chunks {
        merged.add_chunk(chunk);
    }

    merge_hybrid_entities_and_relationships(&mut merged, &local, &global);

    merged
}

fn merge_hybrid_chunks(
    local: &[RetrievedChunk],
    global: &[RetrievedChunk],
    naive: &[RetrievedChunk],
    max_chunks: usize,
    mode: HybridFusionMode,
) -> Vec<RetrievedChunk> {
    if max_chunks == 0 {
        return Vec::new();
    }

    match mode {
        HybridFusionMode::Rrf => {
            let mut lookup: HashMap<String, RetrievedChunk> = HashMap::new();
            for chunk in local.iter().chain(global.iter()).chain(naive.iter()) {
                lookup
                    .entry(chunk.id.clone())
                    .or_insert_with(|| chunk.clone());
            }
            let lists = [
                local.iter().map(|c| c.id.clone()).collect(),
                global.iter().map(|c| c.id.clone()).collect(),
                naive.iter().map(|c| c.id.clone()).collect(),
            ];
            fusion::chunks_from_rrf_ranking(
                &fusion::reciprocal_rank_fusion(&lists, &[1.0, 1.0, 1.0], fusion::RRF_K),
                &lookup,
                max_chunks,
            )
        }
        HybridFusionMode::RoundRobin => round_robin_merge_chunks(local, global, naive, max_chunks),
    }
}

/// LightRAG round-robin: at each index, take local then global then naive (dedup by id).
pub fn round_robin_merge_chunks(
    local: &[RetrievedChunk],
    global: &[RetrievedChunk],
    naive: &[RetrievedChunk],
    max_chunks: usize,
) -> Vec<RetrievedChunk> {
    let mut out = Vec::with_capacity(max_chunks.min(local.len() + global.len() + naive.len()));
    let mut seen = HashSet::new();
    let max_len = local.len().max(global.len()).max(naive.len());

    'outer: for i in 0..max_len {
        for source in [local, global, naive] {
            if let Some(c) = source.get(i) {
                if seen.insert(c.id.clone()) {
                    out.push(c.clone());
                    if out.len() >= max_chunks {
                        break 'outer;
                    }
                }
            }
        }
    }
    out
}

fn merge_hybrid_entities_and_relationships(
    merged: &mut QueryContext,
    local: &QueryContext,
    global: &QueryContext,
) {
    let mut seen_entities = HashSet::new();
    let max_entity_len = local.entities.len().max(global.entities.len());
    for i in 0..max_entity_len {
        for source in [&local.entities, &global.entities] {
            if let Some(e) = source.get(i) {
                if seen_entities.insert(e.name.clone()) {
                    merged.add_entity(e.clone());
                }
            }
        }
    }

    let mut seen_rels = HashSet::new();
    for rel in local
        .relationships
        .iter()
        .chain(global.relationships.iter())
    {
        if seen_rels.insert(rel_key(rel)) {
            merged.add_relationship(rel.clone());
        }
    }
}

fn rel_key(rel: &RetrievedRelationship) -> String {
    format!("{}-{}-{}", rel.source, rel.relation_type, rel.target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::RetrievedEntity;

    fn chunk(id: &str, score: f32) -> RetrievedChunk {
        RetrievedChunk::new(id, id, score)
    }

    #[test]
    fn round_robin_kg_first_and_dedup() {
        let local = vec![chunk("shared", 0.9), chunk("local_only", 0.8)];
        let global = vec![chunk("shared", 0.85), chunk("global_only", 0.7)];
        let naive = vec![chunk("naive_only", 0.6)];

        let merged = round_robin_merge_chunks(&local, &global, &naive, 10);
        let ids: Vec<_> = merged.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["shared", "naive_only", "local_only", "global_only"]
        );
    }

    #[test]
    fn round_robin_respects_max_chunks() {
        let local: Vec<_> = (0..5).map(|i| chunk(&format!("l{i}"), 1.0)).collect();
        let global: Vec<_> = (0..5).map(|i| chunk(&format!("g{i}"), 1.0)).collect();
        let naive: Vec<_> = (0..5).map(|i| chunk(&format!("n{i}"), 1.0)).collect();
        let merged = round_robin_merge_chunks(&local, &global, &naive, 3);
        assert_eq!(merged.len(), 3);
    }

    #[test]
    fn entities_union_dedup() {
        let mut local = QueryContext::new();
        local.add_entity(RetrievedEntity::new("A", "X", "a"));
        let mut global = QueryContext::new();
        global.add_entity(RetrievedEntity::new("A", "X", "a"));
        global.add_entity(RetrievedEntity::new("B", "X", "b"));

        let merged = merge_hybrid_contexts(local, global, QueryContext::new(), 20);
        assert_eq!(merged.entities.len(), 2);
    }
}

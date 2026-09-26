//! Shared mention field-merge policy for embed collapse and authority facts.
//!
//! Identity keys stay different on purpose:
//! - embed: document-wide `EntityId`
//! - authority facts: per extraction chunk `{doc}:{chunk}:{node}`
//!
//! This module owns **field** merge only (description, type vote, importance,
//! weight, provenance) so ANN and graph authority do not diverge on values.

use std::collections::{BTreeSet, HashMap};

use crate::merger::{add_type_vote, merge_type_into_entity, resolve_majority_type, WeightPolicy};

/// Prefer the longer description string.
pub fn prefer_longer_description(existing: &mut String, incoming: &str) {
    if incoming.len() > existing.len() {
        *existing = incoming.to_string();
    }
}

/// True when `incoming` should replace the embed-text description segment.
pub fn incoming_description_is_longer(
    existing_embed_text: &str,
    incoming_description: &str,
) -> bool {
    let existing_len = existing_embed_text
        .split_once('\n')
        .map(|(_, d)| d.len())
        .unwrap_or(existing_embed_text.len());
    incoming_description.len() > existing_len
}

/// Merge entity type votes with clamped importance (no artificial 1.0 seed).
pub fn merge_entity_type_vote(
    entity_type: &mut String,
    type_votes: &mut HashMap<String, f64>,
    incoming_type: &str,
    importance: f32,
    node_id: &str,
) {
    merge_type_into_entity(
        entity_type,
        type_votes,
        incoming_type,
        importance.clamp(0.05, 1.0),
        node_id,
    );
}

/// Seed a fresh type ballot with an importance-weighted vote only.
pub fn seed_entity_type_votes(
    incoming_type: &str,
    importance: f32,
) -> (String, HashMap<String, f64>) {
    let mut type_votes = HashMap::new();
    add_type_vote(&mut type_votes, incoming_type, importance.clamp(0.05, 1.0));
    let entity_type = resolve_majority_type(&type_votes, incoming_type);
    (entity_type, type_votes)
}

/// Max importance across mentions.
pub fn merge_importance(existing: f32, incoming: f32) -> f32 {
    existing.max(incoming)
}

/// Relationship weight via the same SSOT as the merger.
pub fn merge_relationship_weight(existing: f32, incoming: f32) -> f32 {
    WeightPolicy::from_env().combine(existing.max(0.0), incoming.max(0.0))
}

/// Union provenance chunk ids (skip empty).
pub fn union_chunk_ids(into: &mut BTreeSet<String>, extraction_chunk: &str, extra: &[String]) {
    if !extraction_chunk.is_empty() {
        into.insert(extraction_chunk.to_string());
    }
    for cid in extra {
        if !cid.is_empty() {
            into.insert(cid.clone());
        }
    }
}

/// Prefer a filled optional string over empty/None.
pub fn prefer_filled_option(existing: &mut Option<String>, incoming: &Option<String>) {
    let needs_fill = existing
        .as_ref()
        .map(|value| value.is_empty())
        .unwrap_or(true);
    if needs_fill {
        if let Some(value) = incoming {
            if !value.is_empty() {
                *existing = Some(value.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn longer_description_wins() {
        let mut desc = "short".to_string();
        prefer_longer_description(&mut desc, "much longer description");
        assert_eq!(desc, "much longer description");
    }

    #[test]
    fn weight_merge_uses_max_by_default() {
        assert!((merge_relationship_weight(0.4, 0.9) - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn type_seed_is_importance_weighted() {
        let (ty, votes) = seed_entity_type_votes("PERSON", 0.8);
        assert_eq!(ty, "PERSON");
        assert!((votes.get("PERSON").copied().unwrap_or(0.0) - 0.8).abs() < 1e-6);
    }
}

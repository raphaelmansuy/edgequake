//! Within-document unique keys for entity/relationship embedding (SPEC-047 P6).
//!
//! # First principle (LightRAG code-is-law)
//!
//! LightRAG embeds **after** merge: one vector per unique entity/relationship name,
//! not one vector per chunk mention. EdgeQuake previously embedded every mention
//! then deduped in the merger — paying O(Σ mentions) embed cost for O(unique) value.
//!
//! This module is the single source of truth for "which mentions share an embed key"
//! so `generate_all_embeddings` and merger vector collection stay DRY.

use std::collections::HashMap;

use edgequake_storage::EntityId;

use crate::extractor::{ExtractedEntity, ExtractedRelationship, ExtractionResult};
use crate::pipeline::helpers::mention_merge::incoming_description_is_longer;

/// LightRAG entity VDB content: `{name}\n{description}`.
pub fn entity_embed_text(name: &str, description: &str) -> String {
    format!("{name}\n{description}")
}

/// LightRAG relationship VDB content: `{keywords}\t{src}->{tgt}\n{description}`.
pub fn relationship_embed_text(rel: &ExtractedRelationship) -> String {
    format!(
        "{}\t{}->{}\n{}",
        rel.keywords.join(", "),
        rel.source,
        rel.target,
        rel.description
    )
}

/// One unique entity to embed, plus every mention index that should receive the vector.
#[derive(Debug, Clone)]
pub struct UniqueEntityEmbed {
    pub key: String,
    pub text: String,
    /// `(extraction_idx, entity_idx)` for every mention of this entity.
    pub mentions: Vec<(usize, usize)>,
}

/// One unique relationship to embed, plus every mention index.
#[derive(Debug, Clone)]
pub struct UniqueRelationshipEmbed {
    pub key: String,
    pub text: String,
    pub mentions: Vec<(usize, usize)>,
}

/// Collapse entity mentions to unique `EntityId` keys (longer description wins).
///
/// Mentions with empty normalized names are skipped (same as merger W-03).
pub fn unique_entities_for_embed(extractions: &[ExtractionResult]) -> Vec<UniqueEntityEmbed> {
    let mut order: Vec<String> = Vec::new();
    let mut by_key: HashMap<String, UniqueEntityEmbed> = HashMap::new();

    for (ext_idx, extraction) in extractions.iter().enumerate() {
        for (ent_idx, entity) in extraction.entities.iter().enumerate() {
            let entity_id = EntityId::new(&entity.name);
            if entity_id.is_empty() {
                continue;
            }
            let key = entity_id.as_graph_node_id().to_string();
            if let Some(existing) = by_key.get_mut(&key) {
                existing.mentions.push((ext_idx, ent_idx));
                // Prefer richer description for the canonical embed text.
                if incoming_description_is_longer(&existing.text, &entity.description) {
                    existing.text = entity_embed_text(&entity.name, &entity.description);
                }
            } else {
                order.push(key.clone());
                by_key.insert(
                    key,
                    UniqueEntityEmbed {
                        key: entity_id.as_graph_node_id().to_string(),
                        text: entity_embed_text(&entity.name, &entity.description),
                        mentions: vec![(ext_idx, ent_idx)],
                    },
                );
            }
        }
    }

    order
        .into_iter()
        .filter_map(|k| by_key.remove(&k))
        .collect()
}

fn description_len_from_embed_text(text: &str) -> usize {
    text.split_once('\n')
        .map(|(_, d)| d.len())
        .unwrap_or(text.len())
}

/// Collapse relationship mentions to unique `(src, tgt, type)` keys.
pub fn unique_relationships_for_embed(
    extractions: &[ExtractionResult],
) -> Vec<UniqueRelationshipEmbed> {
    let mut order: Vec<String> = Vec::new();
    let mut by_key: HashMap<String, UniqueRelationshipEmbed> = HashMap::new();

    for (ext_idx, extraction) in extractions.iter().enumerate() {
        for (rel_idx, rel) in extraction.relationships.iter().enumerate() {
            let source_id = EntityId::new(&rel.source);
            let target_id = EntityId::new(&rel.target);
            let source_key = source_id.as_graph_node_id();
            let target_key = target_id.as_graph_node_id();
            if source_key.is_empty() || target_key.is_empty() || source_key == target_key {
                continue;
            }
            // SPEC-098 LAW-098-3/8: casefold so "knows"/"KNOWS" collapse.
            let rel_type = edgequake_storage::normalize_relation_type_str(&rel.relation_type);
            let key = format!("{}->{}:{}", source_key, target_key, rel_type);
            if let Some(existing) = by_key.get_mut(&key) {
                existing.mentions.push((ext_idx, rel_idx));
                if rel.description.len() > description_len_from_embed_text(&existing.text) {
                    existing.text = relationship_embed_text(rel);
                }
            } else {
                order.push(key.clone());
                by_key.insert(
                    key.clone(),
                    UniqueRelationshipEmbed {
                        key,
                        text: relationship_embed_text(rel),
                        mentions: vec![(ext_idx, rel_idx)],
                    },
                );
            }
        }
    }

    order
        .into_iter()
        .filter_map(|k| by_key.remove(&k))
        .collect()
}

/// Within-doc entity merge used by the merger vector collector (DRY with embed keys).
///
/// Returns one representative entity per `EntityId`, preferring longer description
/// and any available embedding.
pub fn dedupe_entities_by_id(entities: &[ExtractedEntity]) -> Vec<ExtractedEntity> {
    let mut order: Vec<String> = Vec::new();
    let mut by_key: HashMap<String, ExtractedEntity> = HashMap::new();

    for entity in entities {
        let entity_id = EntityId::new(&entity.name);
        if entity_id.is_empty() {
            continue;
        }
        let key = entity_id.as_graph_node_id().to_string();
        if let Some(existing) = by_key.get_mut(&key) {
            if entity.description.len() > existing.description.len() {
                existing.description = entity.description.clone();
                existing.name = entity.name.clone();
            }
            if existing.embedding.is_none() && entity.embedding.is_some() {
                existing.embedding = entity.embedding.clone();
            }
            for cid in &entity.source_chunk_ids {
                if !existing.source_chunk_ids.contains(cid) {
                    existing.source_chunk_ids.push(cid.clone());
                }
            }
        } else {
            order.push(key.clone());
            by_key.insert(key, entity.clone());
        }
    }

    order
        .into_iter()
        .filter_map(|k| by_key.remove(&k))
        .collect()
}

/// Within-doc relationship merge for vector collection.
pub fn dedupe_relationships_by_endpoints(
    relationships: &[ExtractedRelationship],
) -> Vec<ExtractedRelationship> {
    let mut order: Vec<String> = Vec::new();
    let mut by_key: HashMap<String, ExtractedRelationship> = HashMap::new();

    for rel in relationships {
        let source_id = EntityId::new(&rel.source);
        let target_id = EntityId::new(&rel.target);
        let source_key = source_id.as_graph_node_id();
        let target_key = target_id.as_graph_node_id();
        if source_key.is_empty() || target_key.is_empty() || source_key == target_key {
            continue;
        }
        // SPEC-098 LAW-098-3/8: casefold so "knows"/"KNOWS" collapse.
        let rel_type = edgequake_storage::normalize_relation_type_str(&rel.relation_type);
        let key = format!("{}->{}:{}", source_key, target_key, rel_type);
        if let Some(existing) = by_key.get_mut(&key) {
            if rel.description.len() > existing.description.len() {
                existing.description = rel.description.clone();
            }
            if existing.embedding.is_none() && rel.embedding.is_some() {
                existing.embedding = rel.embedding.clone();
            }
            for kw in &rel.keywords {
                if !existing.keywords.contains(kw) {
                    existing.keywords.push(kw.clone());
                }
            }
            // 049: union chunk lineage (same law as merger endpoint dedupe).
            for cid in rel.all_source_chunk_ids() {
                existing.add_source_chunk_id(cid);
            }
        } else {
            order.push(key.clone());
            by_key.insert(key, rel.clone());
        }
    }

    order
        .into_iter()
        .filter_map(|k| by_key.remove(&k))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractor::ExtractionResult;

    fn entity(name: &str, desc: &str) -> ExtractedEntity {
        ExtractedEntity {
            name: name.to_string(),
            entity_type: "CONCEPT".to_string(),
            description: desc.to_string(),
            importance: 0.5,
            source_spans: vec![],
            embedding: None,
            source_chunk_ids: vec![],
            source_document_id: None,
            source_file_path: None,
            display_name: None,
            page_num: None,
            figure_index: None,
            asset_id: None,
            mm_subtype: None,
        }
    }

    fn extraction(chunk: &str, entities: Vec<ExtractedEntity>) -> ExtractionResult {
        let mut r = ExtractionResult::new(chunk);
        r.entities = entities;
        r
    }

    #[test]
    fn unique_entities_collapse_mentions_and_prefer_longer_description() {
        let extractions = vec![
            extraction("c0", vec![entity("Alpha Beta", "short")]),
            extraction(
                "c1",
                vec![entity(
                    "alpha beta",
                    "a much longer description for quality",
                )],
            ),
        ];
        let unique = unique_entities_for_embed(&extractions);
        assert_eq!(unique.len(), 1);
        assert_eq!(unique[0].mentions.len(), 2);
        assert!(unique[0].text.contains("much longer"));
        assert!(
            unique[0].text.starts_with("alpha beta\n")
                || unique[0].text.starts_with("Alpha Beta\n")
        );
    }

    #[test]
    fn unique_relationships_collapse_same_endpoints() {
        let rel = |src: &str, tgt: &str, desc: &str| ExtractedRelationship {
            source: src.into(),
            target: tgt.into(),
            relation_type: "RELATED".into(),
            description: desc.into(),
            keywords: vec!["k".into()],
            weight: 1.0,
            embedding: None,
            source_chunk_ids: Vec::new(),
            source_chunk_id: None,
            source_document_id: None,
            source_file_path: None,
        };
        let mut extractions = vec![ExtractionResult::new("c0")];
        extractions[0].relationships = vec![
            rel("A", "B", "short"),
            rel("A", "B", "longer relationship description"),
        ];
        let unique = unique_relationships_for_embed(&extractions);
        assert_eq!(unique.len(), 1);
        assert_eq!(unique[0].mentions.len(), 2);
        assert!(unique[0].text.contains("longer relationship"));
    }
}

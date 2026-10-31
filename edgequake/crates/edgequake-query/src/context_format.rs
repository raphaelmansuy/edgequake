//! Context string formatting for LLM prompts (SPEC-047 Q1.1 / 021 F2–F4).
//!
//! # SOLID
//!
//! - **SRP:** Pure formatting of retrieved items for the prompt — no retrieval I/O.
//! - **OCP:** New header fields extend [`format_chunk_header`] without changing callers.
//! - **DIP:** [`QueryContext::to_context_string`] depends on this module, not the reverse.
//!
//! # DRY
//!
//! Truncation token estimates should use the same chunk header format so budget
//! accounting matches what the LLM actually sees.
//!
//! # Modes (021)
//!
//! - `EDGEQUAKE_CONTEXT_FORMAT=flat` (default) — Entities → Relations → Chunks
//! - `EDGEQUAKE_CONTEXT_FORMAT=path` — path-serialized entity–relation–entity blocks
//! - `EDGEQUAKE_CONTEXT_FORMAT=rr_cer` — Relations → Entities → Chunks (028 A1 / Complex)
//! - `EDGEQUAKE_PASSAGE_PACK=1` — chunks-first compact graph (HippoRAG2-style labeled)

use crate::context::{RetrievedChunk, RetrievedEntity, RetrievedRelationship};
use std::collections::{HashMap, HashSet};

/// Prompt layout mode (Acc fairness default = flat).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContextFormatMode {
    #[default]
    Flat,
    /// Path-serialized multi-hop packing (021 F2 / `path_pack_v1`).
    Path,
    /// Relation-first packing for Complex Acc (028 A1).
    ///
    /// LightRAG Mix surfaces relational structure early in the prompt; EQ flat
    /// puts entities first. Order: Relations → Entities → Chunks.
    RrCer,
}

impl ContextFormatMode {
    /// Parse `EDGEQUAKE_CONTEXT_FORMAT` (`flat` \| `path` \| `rr_cer`). Default `flat`.
    pub fn from_env() -> Self {
        match std::env::var("EDGEQUAKE_CONTEXT_FORMAT")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "path" | "paths" | "path_pack" => Self::Path,
            "rr_cer" | "rr-cer" | "relation_first" | "rel_first" => Self::RrCer,
            _ => Self::Flat,
        }
    }
}

/// HippoRAG2-style passage-first packing (021 F4). Default off.
pub fn passage_pack_enabled() -> bool {
    matches!(
        std::env::var("EDGEQUAKE_PASSAGE_PACK")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// LightRAG-like content headings in chunk headers (022 P2). Default off.
pub fn content_headings_enabled() -> bool {
    matches!(
        std::env::var("EDGEQUAKE_CONTENT_HEADINGS")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Build the metadata suffix for a chunk header (`page=N`, `doc=…`, `modality=…`).
///
/// Empty when no grounding metadata is present.
pub fn format_chunk_meta(chunk: &RetrievedChunk) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(page) = chunk.page_start {
        parts.push(format!("page={page}"));
    }
    // SPEC-142: always emit human title when known (disambiguation hint; LLM must not copy).
    if let Some(name) = chunk
        .document_name
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        let safe = name.replace('"', "'");
        parts.push(format!("doc=\"{safe}\""));
    } else if content_headings_enabled() {
        if let Some(doc) = chunk.document_id.as_ref() {
            let trimmed = doc.trim();
            if !trimmed.is_empty() {
                parts.push(format!("doc={trimmed}"));
            }
        }
    }
    if let Some(modality) = chunk.modality.as_ref() {
        let trimmed = modality.trim();
        if !trimmed.is_empty() {
            parts.push(format!("modality={trimmed}"));
        }
    }
    if content_headings_enabled() {
        if let Some(idx) = chunk.chunk_index {
            parts.push(format!("heading=chunk-{idx}"));
        }
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" {}", parts.join(" "))
    }
}

/// Assign stable `citation_id` = 1..n when unset (SPEC-083 X-20).
///
/// Never renumber after format — callers must stamp before prompt/API emit.
pub fn assign_stable_citation_ids(chunks: &mut [RetrievedChunk]) {
    let mut used = chunks.iter().filter_map(|c| c.citation_id).collect();
    assign_stable_citation_ids_avoiding(chunks, &mut used);
}

pub(crate) fn assign_stable_citation_ids_avoiding(
    chunks: &mut [RetrievedChunk],
    used: &mut std::collections::HashSet<usize>,
) {
    let mut next = 1;
    for chunk in chunks.iter_mut() {
        if chunk.citation_id.is_none() {
            while used.contains(&next) {
                next += 1;
            }
            chunk.citation_id = Some(next);
            used.insert(next);
        }
    }
}

/// Resolve the citation index for a chunk (stable id, else positional fallback).
pub fn chunk_citation_ref(chunk: &RetrievedChunk, fallback: usize) -> usize {
    chunk.citation_id.unwrap_or(fallback)
}

/// One chunk block as injected into the LLM context.
///
/// Example: `[1] (score: 0.850) page=12 modality=chart\n…content…\n\n`
pub fn format_chunk_block(ref_id: usize, chunk: &RetrievedChunk) -> String {
    let meta = format_chunk_meta(chunk);
    let ref_id = chunk_citation_ref(chunk, ref_id);
    format!(
        "[{ref_id}] (score: {:.3}){meta}\n{}\n\n",
        chunk.score,
        chunk_body(chunk)
    )
}

/// The text the model sees for a chunk. An image chunk has no text (SPEC-180): it is
/// named, so the model can cite the picture without inventing what it shows.
pub fn chunk_body(chunk: &RetrievedChunk) -> String {
    let is_image = chunk.modality.as_deref() == Some("image");
    if !is_image || !chunk.content.trim().is_empty() {
        return chunk.content.clone();
    }
    match chunk.document_name.as_deref().map(str::trim) {
        Some(name) if !name.is_empty() => format!("[Image: {name}]"),
        _ => "[Image]".to_string(),
    }
}

/// Format string used when estimating entity tokens (must stay close to prompt).
pub fn format_entity_line(entity: &RetrievedEntity) -> String {
    let degree_info = if entity.degree > 0 {
        format!(" [connections: {}]", entity.degree)
    } else {
        String::new()
    };
    format!(
        "- **{}** ({}){}: {}\n",
        entity.name, entity.entity_type, degree_info, entity.description
    )
}

/// Format string used when estimating relationship tokens.
pub fn format_relationship_line(rel: &RetrievedRelationship) -> String {
    // 073: use presentation labels so LLM context never embeds bare UUIDs.
    let src = rel.display_source();
    let tgt = rel.display_target();
    if rel.description.is_empty() {
        format!("- {} --[{}]--> {}\n", src, rel.relation_type, tgt)
    } else {
        format!(
            "- {} --[{}]--> {}: {}\n",
            src, rel.relation_type, tgt, rel.description
        )
    }
}

fn chunk_legend() -> &'static str {
    "Each chunk header may include `page=N` (1-indexed PDF page), `doc=\"Title\"` \
(document name — do not copy into the answer; cite [N] only), and `modality=` \
(chart/figure/table/equation when set). Prefer evidence from matching pages/modalities when answering.\n\n"
}

fn format_chunks_section(chunks: &[RetrievedChunk], start_ref: usize) -> String {
    if chunks.is_empty() {
        return String::new();
    }
    let mut parts = Vec::new();
    parts.push("### Chunks\n\n".to_string());
    parts.push(chunk_legend().to_string());
    for (i, chunk) in chunks.iter().enumerate() {
        // X-20: prefer stable citation_id over positional i+1.
        parts.push(format_chunk_block(
            chunk_citation_ref(chunk, start_ref + i),
            chunk,
        ));
    }
    parts.join("")
}

fn format_entities_section(entities: &[RetrievedEntity]) -> String {
    if entities.is_empty() {
        return String::new();
    }
    let mut parts = Vec::new();
    parts.push("### Entities\n\n".to_string());
    for entity in entities {
        parts.push(format_entity_line(entity));
    }
    parts.push("\n".to_string());
    parts.join("")
}

fn format_relations_section(relationships: &[RetrievedRelationship]) -> String {
    if relationships.is_empty() {
        return String::new();
    }
    let mut parts = Vec::new();
    parts.push("### Relations\n\n".to_string());
    for rel in relationships {
        parts.push(format_relationship_line(rel));
    }
    parts.push("\n".to_string());
    parts.join("")
}

/// Flat layout: Entities → Relations → Chunks (LightRAG-aligned section titles).
pub fn format_query_context_flat(
    entities: &[RetrievedEntity],
    relationships: &[RetrievedRelationship],
    chunks: &[RetrievedChunk],
) -> String {
    [
        format_entities_section(entities),
        format_relations_section(relationships),
        format_chunks_section(chunks, 1),
    ]
    .join("")
}

/// Relation-first layout (028 A1): Relations → Entities → Chunks.
///
/// Surfaces multi-hop edges before hub entities so Complex reasoning sees
/// relational structure earlier in the prompt budget.
pub fn format_query_context_rr_cer(
    entities: &[RetrievedEntity],
    relationships: &[RetrievedRelationship],
    chunks: &[RetrievedChunk],
) -> String {
    [
        "### Context layout\n\nRelations first (multi-hop edges), then entities, then supporting chunks.\n\n"
            .to_string(),
        format_relations_section(relationships),
        format_entities_section(entities),
        format_chunks_section(chunks, 1),
    ]
    .join("")
}

/// Path-serialized multi-hop packing (021 F2).
///
/// Emits `### Reasoning Paths` blocks: entity --[rel]--> entity with supporting
/// chunk snippets linked via `source_chunk_ids`, then remaining graph + chunks.
pub fn format_query_context_path(
    entities: &[RetrievedEntity],
    relationships: &[RetrievedRelationship],
    chunks: &[RetrievedChunk],
) -> String {
    let entity_by_name: HashMap<String, &RetrievedEntity> = entities
        .iter()
        .map(|e| (e.name.to_ascii_uppercase(), e))
        .collect();
    let chunk_by_id: HashMap<&str, &RetrievedChunk> =
        chunks.iter().map(|c| (c.id.as_str(), c)).collect();

    let mut parts = Vec::new();
    let mut used_chunk_ids: HashSet<String> = HashSet::new();
    let mut used_entity_names: HashSet<String> = HashSet::new();
    let mut used_rel_keys: HashSet<String> = HashSet::new();
    let mut path_ref = 1usize;

    if !relationships.is_empty() {
        parts.push("### Reasoning Paths\n\n".to_string());
        parts.push(
            "Each path links entities via a relation, then lists supporting chunks. \
Use paths for multi-hop reasoning; prefer cited chunk evidence.\n\n"
                .to_string(),
        );

        for rel in relationships {
            let key = format!(
                "{}|{}|{}",
                rel.source.to_ascii_uppercase(),
                rel.relation_type.to_ascii_uppercase(),
                rel.target.to_ascii_uppercase()
            );
            if !used_rel_keys.insert(key) {
                continue;
            }

            let src_key = rel.source.to_ascii_uppercase();
            let tgt_key = rel.target.to_ascii_uppercase();
            used_entity_names.insert(src_key.clone());
            used_entity_names.insert(tgt_key.clone());

            parts.push(format!("#### Path {path_ref}\n"));
            path_ref += 1;
            parts.push(format_relationship_line(rel));

            if let Some(src) = entity_by_name.get(&src_key) {
                parts.push(format!("- Source: {}", format_entity_line(src).trim()));
                parts.push("\n".to_string());
            }
            if let Some(tgt) = entity_by_name.get(&tgt_key) {
                parts.push(format!("- Target: {}", format_entity_line(tgt).trim()));
                parts.push("\n".to_string());
            }

            let mut support: Vec<&RetrievedChunk> = Vec::new();
            for name_key in [&src_key, &tgt_key] {
                if let Some(ent) = entity_by_name.get(name_key) {
                    for cid in &ent.source_chunk_ids {
                        if let Some(ch) = chunk_by_id.get(cid.as_str()) {
                            if used_chunk_ids.insert(ch.id.clone()) {
                                support.push(*ch);
                            }
                        }
                    }
                }
            }
            // Cap support snippets per path to keep prompt compact.
            support.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            support.truncate(3);
            if !support.is_empty() {
                parts.push("Supporting chunks:\n".to_string());
                for (i, ch) in support.iter().enumerate() {
                    parts.push(format_chunk_block(i + 1, ch));
                }
            }
            parts.push("\n".to_string());
        }
    }

    let leftover_entities: Vec<&RetrievedEntity> = entities
        .iter()
        .filter(|e| !used_entity_names.contains(&e.name.to_ascii_uppercase()))
        .collect();
    if !leftover_entities.is_empty() {
        parts.push("### Entities\n\n".to_string());
        for entity in leftover_entities {
            parts.push(format_entity_line(entity));
        }
        parts.push("\n".to_string());
    }

    // Relations already listed under Reasoning Paths — no duplicate ### Relations.

    let leftover_chunks: Vec<&RetrievedChunk> = chunks
        .iter()
        .filter(|c| !used_chunk_ids.contains(&c.id))
        .collect();
    if !leftover_chunks.is_empty() {
        parts.push("### Chunks\n\n".to_string());
        parts.push(chunk_legend().to_string());
        for (i, chunk) in leftover_chunks.iter().enumerate() {
            parts.push(format_chunk_block(i + 1, chunk));
        }
    } else if used_chunk_ids.is_empty() && !chunks.is_empty() {
        // No path-linked chunks — fall back to full chunk section.
        parts.push(format_chunks_section(chunks, 1));
    }

    parts.join("")
}

/// Passage-first compact packing (021 F4 / HippoRAG2-inspired).
///
/// Chunks lead; entities/relations follow as compact graph evidence.
pub fn format_query_context_passage_pack(
    entities: &[RetrievedEntity],
    relationships: &[RetrievedRelationship],
    chunks: &[RetrievedChunk],
) -> String {
    let mut parts = Vec::new();
    parts.push(format_chunks_section(chunks, 1));
    // Compact graph: cap to top-scored entities / relations already in context order.
    let ent_cap = entities.len().min(12);
    let rel_cap = relationships.len().min(16);
    parts.push(format_entities_section(&entities[..ent_cap]));
    parts.push(format_relations_section(&relationships[..rel_cap]));
    parts.join("")
}

/// Assemble the full context string using env-selected layout.
pub fn format_query_context(
    entities: &[RetrievedEntity],
    relationships: &[RetrievedRelationship],
    chunks: &[RetrievedChunk],
) -> String {
    format_query_context_with_mode(
        entities,
        relationships,
        chunks,
        ContextFormatMode::from_env(),
        passage_pack_enabled(),
    )
}

/// Explicit mode (tests / callers that avoid env).
pub fn format_query_context_with_mode(
    entities: &[RetrievedEntity],
    relationships: &[RetrievedRelationship],
    chunks: &[RetrievedChunk],
    mode: ContextFormatMode,
    passage_pack: bool,
) -> String {
    if passage_pack {
        return format_query_context_passage_pack(entities, relationships, chunks);
    }
    match mode {
        ContextFormatMode::Flat => format_query_context_flat(entities, relationships, chunks),
        ContextFormatMode::Path => format_query_context_path(entities, relationships, chunks),
        ContextFormatMode::RrCer => format_query_context_rr_cer(entities, relationships, chunks),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{RetrievedChunk, RetrievedEntity, RetrievedRelationship};

    #[test]
    fn chunk_meta_includes_page_and_modality() {
        let chunk = RetrievedChunk::new("c1", "Revenue rose 12%", 0.91)
            .with_page(7)
            .with_modality("chart");
        let meta = format_chunk_meta(&chunk);
        assert!(meta.contains("page=7"));
        assert!(meta.contains("modality=chart"));
    }

    #[test]
    fn chunk_block_header_is_groundable() {
        let chunk = RetrievedChunk::new("c1", "Table row A", 0.8)
            .with_page(3)
            .with_modality("table");
        let block = format_chunk_block(1, &chunk);
        assert!(block.starts_with("[1] (score: 0.800) page=3 modality=table\n"));
        assert!(block.contains("Table row A"));
    }

    #[test]
    fn format_query_context_mentions_page_legend() {
        let chunk = RetrievedChunk::new("c1", "body", 0.5).with_page(2);
        let s = format_query_context_flat(
            &[RetrievedEntity::new("Acme", "ORG", "A company")],
            &[],
            &[chunk],
        );
        assert!(s.contains("page=2"));
        assert!(s.contains("page=N"));
        assert!(s.contains("### Chunks"));
    }

    #[test]
    fn path_format_emits_reasoning_paths() {
        let mut a = RetrievedEntity::new("ALPHA", "ORG", "Source org");
        a.source_chunk_ids = vec!["c1".into()];
        let mut b = RetrievedEntity::new("BETA", "ORG", "Target org");
        b.source_chunk_ids = vec!["c1".into()];
        let rel = RetrievedRelationship::new("ALPHA", "BETA", "PARTNERS_WITH")
            .with_description("works with");
        let chunk = RetrievedChunk::new("c1", "Alpha partners with Beta", 0.9);
        let s = format_query_context_path(&[a, b], &[rel], &[chunk]);
        assert!(s.contains("### Reasoning Paths"));
        assert!(s.contains("PARTNERS_WITH"));
        assert!(s.contains("Supporting chunks"));
        assert!(s.contains("Alpha partners with Beta"));
    }

    #[test]
    fn passage_pack_puts_chunks_first() {
        let ent = RetrievedEntity::new("Acme", "ORG", "A company");
        let chunk = RetrievedChunk::new("c1", "body text", 0.5);
        let s = format_query_context_passage_pack(&[ent], &[], &[chunk]);
        let chunks_pos = s.find("### Chunks").expect("chunks");
        let ents_pos = s.find("### Entities").expect("entities");
        assert!(chunks_pos < ents_pos);
    }

    #[test]
    fn rr_cer_puts_relations_before_entities() {
        let ent = RetrievedEntity::new("ALPHA", "ORG", "Source org");
        let rel = RetrievedRelationship::new("ALPHA", "BETA", "PARTNERS_WITH")
            .with_description("works with");
        let chunk = RetrievedChunk::new("c1", "body", 0.5);
        let s = format_query_context_rr_cer(&[ent], &[rel], &[chunk]);
        let rel_pos = s.find("### Relations").expect("relations");
        let ent_pos = s.find("### Entities").expect("entities");
        let chunk_pos = s.find("### Chunks").expect("chunks");
        assert!(rel_pos < ent_pos && ent_pos < chunk_pos);
        assert!(s.contains("PARTNERS_WITH"));
    }

    #[test]
    fn relationship_line_uses_soft_labels_not_raw_uuid() {
        let opaque = "84B69E27-E38B-444A-83DD-5E6A537C6F12";
        let mut rel = RetrievedRelationship::new(opaque, "AI_NEXT_CONFERENCE", "HAS_THEME");
        rel.source_label = "Future of work theme from the agenda".into();
        rel.target_label = "AI Next Conference".into();
        let line = format_relationship_line(&rel);
        assert!(line.contains("Future of work"), "got {line}");
        assert!(line.contains("AI Next Conference"), "got {line}");
        assert!(!line.contains("84B69E27"), "must not embed UUID: {line}");
    }

    #[test]
    fn citation_assignment_avoids_preserved_ids() {
        let mut chunks = vec![
            RetrievedChunk::new("a", "first", 0.9).with_citation_id(2),
            RetrievedChunk::new("b", "second", 0.8),
            RetrievedChunk::new("c", "third", 0.7).with_citation_id(usize::MAX),
        ];
        assign_stable_citation_ids(&mut chunks);
        assert_eq!(chunks[0].citation_id, Some(2));
        assert_eq!(chunks[1].citation_id, Some(1));
        assert_eq!(chunks[2].citation_id, Some(usize::MAX));
    }

    #[test]
    fn dual_citation_assignment_reserves_existing_and_reuses_chunk_ids() {
        let mut ctx = crate::context::QueryContext::new();
        ctx.chunks = vec![RetrievedChunk::new("a", "prompt", 0.9)];
        ctx.citation_chunks = Some(vec![
            RetrievedChunk::new("b", "citation", 0.8).with_citation_id(1),
            RetrievedChunk::new("c", "citation", 0.7),
            RetrievedChunk::new("c", "citation", 0.7),
            RetrievedChunk::new("a", "prompt", 0.9),
        ]);
        ctx.ensure_stable_citation_ids();
        assert_eq!(ctx.chunks[0].citation_id, Some(2));
        let stamped = ctx
            .citation_chunks
            .as_ref()
            .unwrap()
            .iter()
            .map(|c| c.citation_id)
            .collect::<Vec<_>>();
        ctx.ensure_stable_citation_ids();
        assert_eq!(
            ctx.citation_chunks
                .as_ref()
                .unwrap()
                .iter()
                .map(|c| c.citation_id)
                .collect::<Vec<_>>(),
            stamped
        );
        let citations = ctx.citation_chunks.unwrap();
        assert_eq!(
            citations.iter().map(|c| c.citation_id).collect::<Vec<_>>(),
            vec![Some(1), Some(3), Some(3), Some(2)]
        );
    }

    #[test]
    fn dual_citation_assignment_handles_maximum_preserved_id() {
        let mut ctx = crate::context::QueryContext::new();
        ctx.chunks = vec![RetrievedChunk::new("a", "prompt", 0.9).with_citation_id(usize::MAX)];
        ctx.citation_chunks = Some(vec![RetrievedChunk::new("b", "citation", 0.8)]);
        ctx.ensure_stable_citation_ids();
        assert_eq!(ctx.chunks[0].citation_id, Some(usize::MAX));
        assert_eq!(ctx.citation_chunks.unwrap()[0].citation_id, Some(1));
    }

    /// SPEC-083 X-20: prompt [N] follows citation_id, not list position after reorder.
    #[test]
    fn contract_citation_stable_ids() {
        let mut chunks = vec![
            RetrievedChunk::new("a", "first", 0.9),
            RetrievedChunk::new("b", "second", 0.8),
            RetrievedChunk::new("c", "third", 0.7),
        ];
        assign_stable_citation_ids(&mut chunks);
        assert_eq!(chunks[0].citation_id, Some(1));
        assert_eq!(chunks[1].citation_id, Some(2));
        assert_eq!(chunks[2].citation_id, Some(3));

        // Reorder after stamp — [N] must follow citation_id, not new position.
        // After swap(0,2): c@id=3 score=0.7 first, a@id=1 score=0.9 last.
        chunks.swap(0, 2);
        let s = format_query_context_flat(&[], &[], &chunks);
        assert!(
            s.contains("[3] (score: 0.700)") && s.contains("[1] (score: 0.900)"),
            "stable citation_id must survive reorder; got:\n{s}"
        );

        // Contract: sources of truth mention citation_id.
        let fmt_src = include_str!("context_format.rs");
        assert!(
            fmt_src.contains("citation_id") && fmt_src.contains("assign_stable_citation_ids"),
            "X-20: context_format must use citation_id"
        );
        let builder = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../edgequake-api/src/services/source_reference_builder.rs"
        );
        let api_src = std::fs::read_to_string(builder)
            .unwrap_or_else(|e| panic!("read source_reference_builder.rs: {e}"));
        assert!(
            api_src.contains("citation_id"),
            "X-20: API source builder must use citation_id for reference_id"
        );
    }
}

#[cfg(test)]
mod image_chunk_tests {
    use super::*;

    fn image(name: Option<&str>) -> RetrievedChunk {
        let mut chunk = RetrievedChunk::new("c", "", 0.5);
        chunk.modality = Some("image".into());
        chunk.document_name = name.map(str::to_string);
        chunk
    }

    #[test]
    fn an_empty_image_chunk_is_named() {
        assert!(format_chunk_block(1, &image(Some("kestrel.png"))).contains("[Image: kestrel.png]"));
        assert!(format_chunk_block(1, &image(None)).contains("\n[Image]\n"));
    }

    #[test]
    fn text_chunks_are_left_alone() {
        let chunk = RetrievedChunk::new("c", "plain text", 0.5);
        assert!(format_chunk_block(1, &chunk).contains("\nplain text\n"));
    }
}

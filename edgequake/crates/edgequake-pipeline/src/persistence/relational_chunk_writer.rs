//! SPEC-091 W1 — relational chunk writer (single writer site for `chunks` rows).
//! SPEC-118 — injection composite doc ids map to injection UUID via shared resolver.
//! SPEC-149 — canonical fact accumulator: duplicate extraction mentions collapse
//! to one `(fact, contribution)` pair before authority validation.

use std::collections::{BTreeSet, HashMap};

use edgequake_storage::traits::domain::{
    Chunk, ChunkId, ChunkRepository, DocumentId, TenantId, UnitOfWork, WorkspaceId,
};
use edgequake_storage::{normalize_relation_type_str, EntityId, StorageError};
use edgequake_storage_contracts::{
    AccessScope, PreparedIngestionBatch, PreparedRecord, MAX_BATCH_RECORDS,
};
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

use crate::chunker::TextChunk;
use crate::extractor::{ExtractedEntity, ExtractedRelationship};
use crate::pipeline::helpers::mention_merge::{
    merge_entity_type_vote, merge_importance, merge_relationship_weight, prefer_filled_option,
    prefer_longer_description, seed_entity_type_votes, union_chunk_ids,
};
use crate::pipeline::ProcessingResult;

use super::document_id_resolve::{
    is_injection_composite_document_id, resolve_relational_document_id,
};
use super::IngestionPersistContext;

/// Build domain chunks from a processing result (legacy string ids preserved in metadata).
pub fn build_relational_chunks(
    ctx: &IngestionPersistContext,
    result: &ProcessingResult,
) -> Result<Vec<Chunk>, StorageError> {
    let document_id = parse_document_id(&ctx.document_id)?;
    let bridged_injection = is_injection_composite_document_id(&ctx.document_id);
    let tenant_id = ctx
        .tenant_id
        .as_deref()
        .map(parse_uuid)
        .transpose()?
        .map(TenantId::new);
    let workspace_id = ctx
        .workspace_id
        .as_deref()
        .map(parse_uuid)
        .transpose()?
        .map(WorkspaceId::new);

    Ok(result
        .chunks
        .iter()
        .map(|chunk| {
            // Metadata mirrors `chunk_storage::chunk_kv_value` so relational
            // reads can reconstruct the full legacy chunk shape (SSOT cutover).
            let mut metadata = serde_json::json!({
                "legacy_chunk_key": chunk.id,
                "start_line": chunk.start_line,
                "end_line": chunk.end_line,
            });
            // SPEC-118: preserve composite injection artifact id alongside FK UUID.
            if bridged_injection {
                metadata["legacy_document_id"] = serde_json::json!(&ctx.document_id);
            }
            if let Some(file) = ctx.source_file_path.as_deref() {
                metadata["source_file"] = serde_json::json!(file);
            }
            if let Some(section) = &chunk.section {
                metadata["section"] = serde_json::json!({
                    "heading_path": section.heading_path,
                    "heading_level": section.heading_level,
                });
            }
            if let Some(page) = chunk.page_start {
                metadata["page_start"] = serde_json::json!(page);
                metadata["page_end"] = serde_json::json!(chunk.page_end.unwrap_or(page));
            }
            if let Some(modality) = chunk.modality.clone().or_else(|| {
                crate::multimodal::resolve_retrieval_modality_from_content(&chunk.content)
                    .map(str::to_string)
            }) {
                metadata["modality"] = serde_json::json!(modality);
            }
            // SPEC-180: the typed `chunks.source_modality` column is filled from here.
            if chunk.modality.as_deref() == Some(crate::multimodal::MODALITY_IMAGE) {
                metadata["source_modality"] = serde_json::json!("image");
            }
            let stable_key = format!("{}:{}:{}", document_id.into_uuid(), chunk.index, chunk.id);
            Chunk {
                id: ChunkId::new(Uuid::new_v5(&Uuid::NAMESPACE_OID, stable_key.as_bytes())),
                document_id,
                tenant_id,
                workspace_id,
                chunk_index: i32::try_from(chunk.index).unwrap_or(i32::MAX),
                content: chunk.content.clone(),
                start_offset: i32::try_from(chunk.start_offset).ok(),
                end_offset: i32::try_from(chunk.end_offset).ok(),
                token_count: i32::try_from(chunk.token_count).ok(),
                metadata,
                page_start: chunk.page_start.and_then(|p| i32::try_from(p).ok()),
                page_end: chunk.page_end.and_then(|p| i32::try_from(p).ok()),
            }
        })
        .collect())
}

/// Build a single authority command (ordinal 0, expected revision = generation − 1).
///
/// Prefer [`pack_prepared_ingestion_batches`] for production persist so dense
/// documents stay under [`MAX_BATCH_RECORDS`].
pub fn build_prepared_ingestion_batch(
    ctx: &IngestionPersistContext,
    result: &ProcessingResult,
    chunks: &[Chunk],
    embedding_model_id: &str,
    generation: u64,
) -> Result<PreparedIngestionBatch, StorageError> {
    build_prepared_ingestion_batch_at(
        ctx,
        result,
        chunks,
        &result.chunks,
        embedding_model_id,
        generation,
        0,
        Some(generation.saturating_sub(1)),
    )
}

/// SPEC-149: split a processing result into bounded staging commands.
///
/// Packs contiguous relational chunks so each command's
/// `chunks + facts + contributions + embeddings` count is ≤ [`MAX_BATCH_RECORDS`].
/// Ordinal fencing matches the postgres ingestion committer:
/// - ordinal 0 → `expected_revision = generation - 1`
/// - ordinal > 0 → `expected_revision = generation` (revision already advanced)
pub fn pack_prepared_ingestion_batches(
    ctx: &IngestionPersistContext,
    result: &ProcessingResult,
    chunks: &[Chunk],
    embedding_model_id: &str,
    generation: u64,
) -> Result<Vec<PreparedIngestionBatch>, StorageError> {
    if generation == 0 {
        return Err(StorageError::InvalidData(
            "ingest generation must be greater than zero".into(),
        ));
    }
    if chunks.len() != result.chunks.len() {
        return Err(StorageError::InvalidData(format!(
            "relational chunk count {} != source chunk count {}",
            chunks.len(),
            result.chunks.len()
        )));
    }

    if chunks.is_empty() {
        return Ok(vec![build_prepared_ingestion_batch(
            ctx,
            result,
            chunks,
            embedding_model_id,
            generation,
        )?]);
    }

    let known_ids = result
        .chunks
        .iter()
        .map(|chunk| chunk.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    let mut per_chunk_cost = per_chunk_record_costs(ctx, result, generation)?;
    // Orphan extractions (unknown source_chunk_id) ride with ordinal 0.
    let orphan_cost = orphan_extraction_record_cost(ctx, result, &known_ids, generation)?;
    if let Some(first) = per_chunk_cost.first_mut() {
        *first = first.saturating_add(orphan_cost);
    }
    for (index, cost) in per_chunk_cost.iter().enumerate() {
        if *cost > MAX_BATCH_RECORDS {
            return Err(StorageError::InvalidData(format!(
                "chunk index {index} alone requires {cost} records; maximum is {MAX_BATCH_RECORDS}"
            )));
        }
    }

    let total_cost: usize = per_chunk_cost.iter().copied().sum();
    if total_cost <= MAX_BATCH_RECORDS {
        return Ok(vec![build_prepared_ingestion_batch(
            ctx,
            result,
            chunks,
            embedding_model_id,
            generation,
        )?]);
    }

    let ranges = pack_chunk_index_ranges(&per_chunk_cost, MAX_BATCH_RECORDS);

    let mut batches = Vec::with_capacity(ranges.len());
    for (ordinal, (start, end)) in ranges.into_iter().enumerate() {
        let batch_ordinal = u64::try_from(ordinal)
            .map_err(|_| StorageError::InvalidData("batch ordinal exceeds u64".into()))?;
        let expected_revision = if batch_ordinal == 0 {
            Some(generation.saturating_sub(1))
        } else {
            Some(generation)
        };
        let slice_chunks = &chunks[start..end];
        let slice_sources = &result.chunks[start..end];
        let pack_ids = slice_sources
            .iter()
            .map(|chunk| chunk.id.as_str())
            .collect::<std::collections::HashSet<_>>();
        let include_orphans = batch_ordinal == 0;
        let filtered = ProcessingResult {
            document_id: result.document_id.clone(),
            chunks: slice_sources.to_vec(),
            extractions: result
                .extractions
                .iter()
                .filter(|extraction| {
                    let source = extraction.source_chunk_id.as_str();
                    pack_ids.contains(source) || (include_orphans && !known_ids.contains(source))
                })
                .cloned()
                .collect(),
            stats: result.stats.clone(),
            lineage: None,
        };
        batches.push(build_prepared_ingestion_batch_at(
            ctx,
            &filtered,
            slice_chunks,
            slice_sources,
            embedding_model_id,
            generation,
            batch_ordinal,
            expected_revision,
        )?);
    }
    Ok(batches)
}

/// Build the canonical authority command consumed by `IngestionCommitter`.
///
/// Duplicate extraction mentions collapse to one fact/contribution pair keyed
/// by stable logical identity. Record revisions equal `generation` so reprocess
/// advances without colliding with prior immutable rows. IDs and JSON field
/// ordering stay deterministic so a retry produces the same digest.
///
/// `source_chunks` must align 1:1 with `chunks` (embeddings + legacy ids).
#[allow(clippy::too_many_arguments)]
pub fn build_prepared_ingestion_batch_at(
    ctx: &IngestionPersistContext,
    result: &ProcessingResult,
    chunks: &[Chunk],
    source_chunks: &[TextChunk],
    embedding_model_id: &str,
    generation: u64,
    batch_ordinal: u64,
    expected_revision: Option<u64>,
) -> Result<PreparedIngestionBatch, StorageError> {
    let document_id = parse_document_id(&ctx.document_id)?;
    let tenant_id = required_scope_uuid("tenant", ctx.tenant_id.as_deref())?;
    let workspace_id = required_scope_uuid("workspace", ctx.workspace_id.as_deref())?;
    if generation == 0 {
        return Err(StorageError::InvalidData(
            "ingest generation must be greater than zero".into(),
        ));
    }
    if chunks.len() != source_chunks.len() {
        return Err(StorageError::InvalidData(format!(
            "relational chunk count {} != source chunk count {}",
            chunks.len(),
            source_chunks.len()
        )));
    }
    let revision = generation;

    let chunk_records = chunks
        .iter()
        .map(|chunk| prepared_json_record(chunk, revision))
        .collect::<Result<Vec<_>, _>>()?;

    let (facts, contributions) = build_canonical_fact_records(
        document_id.into_uuid(),
        tenant_id,
        workspace_id,
        result,
        revision,
    )?;

    let embeddings = chunks
        .iter()
        .zip(source_chunks)
        .filter_map(|(chunk, source)| {
            source.embedding.as_ref().map(|embedding| {
                let payload = serde_json::json!({
                    "schema": "edgequake.embedding.v1",
                    "family": "chunk",
                    "subject_id": chunk.id.0,
                    "workspace_id": workspace_id,
                    "model_id": embedding_model_id,
                    "dimensions": embedding.len(),
                    "embedding": embedding,
                    "legacy_vector_id": source.id,
                });
                prepared_json_value(chunk.id.0, revision, &payload)
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let scope = AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id));
    let idempotency_key = format!("{}:{generation}:{batch_ordinal}", document_id.into_uuid());
    let canonical = serde_json::to_vec(&serde_json::json!({
        "scope": scope,
        "document_id": document_id.into_uuid(),
        "ingest_generation": generation,
        "batch_ordinal": batch_ordinal,
        "idempotency_key": idempotency_key,
        "schema_version": 1,
        "chunks": chunk_records,
        "facts": facts,
        "contributions": contributions,
        "embeddings": embeddings,
    }))
    .map_err(|error| StorageError::InvalidData(format!("encode ingestion command: {error}")))?;

    Ok(PreparedIngestionBatch {
        scope,
        document_id,
        ingest_generation: generation,
        batch_ordinal,
        expected_revision,
        idempotency_key,
        schema_version: 1,
        canonical_digest: sha256(&canonical),
        chunks: chunk_records,
        facts,
        contributions,
        embeddings,
    })
}

/// Per-chunk admission cost: 1 chunk + facts + contributions + optional embedding.
fn per_chunk_record_costs(
    ctx: &IngestionPersistContext,
    result: &ProcessingResult,
    generation: u64,
) -> Result<Vec<usize>, StorageError> {
    let document_id = parse_document_id(&ctx.document_id)?;
    let tenant_id = required_scope_uuid("tenant", ctx.tenant_id.as_deref())?;
    let workspace_id = required_scope_uuid("workspace", ctx.workspace_id.as_deref())?;
    let revision = generation;
    let mut costs = Vec::with_capacity(result.chunks.len());
    for source in &result.chunks {
        let filtered = ProcessingResult {
            document_id: result.document_id.clone(),
            chunks: vec![source.clone()],
            extractions: result
                .extractions
                .iter()
                .filter(|extraction| extraction.source_chunk_id == source.id)
                .cloned()
                .collect(),
            stats: Default::default(),
            lineage: None,
        };
        let (facts, contributions) = build_canonical_fact_records(
            document_id.into_uuid(),
            tenant_id,
            workspace_id,
            &filtered,
            revision,
        )?;
        let embedding = usize::from(source.embedding.is_some());
        costs.push(
            1usize
                .saturating_add(facts.len())
                .saturating_add(contributions.len())
                .saturating_add(embedding),
        );
    }
    Ok(costs)
}

fn orphan_extraction_record_cost(
    ctx: &IngestionPersistContext,
    result: &ProcessingResult,
    known_ids: &std::collections::HashSet<&str>,
    generation: u64,
) -> Result<usize, StorageError> {
    let orphans: Vec<_> = result
        .extractions
        .iter()
        .filter(|extraction| !known_ids.contains(extraction.source_chunk_id.as_str()))
        .cloned()
        .collect();
    if orphans.is_empty() {
        return Ok(0);
    }
    let document_id = parse_document_id(&ctx.document_id)?;
    let tenant_id = required_scope_uuid("tenant", ctx.tenant_id.as_deref())?;
    let workspace_id = required_scope_uuid("workspace", ctx.workspace_id.as_deref())?;
    let filtered = ProcessingResult {
        document_id: result.document_id.clone(),
        chunks: Vec::new(),
        extractions: orphans,
        stats: Default::default(),
        lineage: None,
    };
    let (facts, contributions) = build_canonical_fact_records(
        document_id.into_uuid(),
        tenant_id,
        workspace_id,
        &filtered,
        generation,
    )?;
    Ok(facts.len().saturating_add(contributions.len()))
}

/// Greedy contiguous packs of chunk indexes under `budget`.
fn pack_chunk_index_ranges(costs: &[usize], budget: usize) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start = 0usize;
    let mut used = 0usize;
    for (index, cost) in costs.iter().enumerate() {
        if start < index && used.saturating_add(*cost) > budget {
            ranges.push((start, index));
            start = index;
            used = 0;
        }
        used = used.saturating_add(*cost);
    }
    if start < costs.len() || costs.is_empty() {
        ranges.push((start, costs.len()));
    }
    ranges
}

/// Ordered canonical entity/relationship facts keyed by stable identity.
fn build_canonical_fact_records(
    document_uuid: Uuid,
    tenant_id: Uuid,
    workspace_id: Uuid,
    result: &ProcessingResult,
    revision: u64,
) -> Result<(Vec<PreparedRecord>, Vec<PreparedRecord>), StorageError> {
    let mut node_order: Vec<String> = Vec::new();
    let mut nodes: HashMap<String, CanonicalNode> = HashMap::new();
    let mut edge_order: Vec<String> = Vec::new();
    let mut edges: HashMap<String, CanonicalEdge> = HashMap::new();

    for extraction in &result.extractions {
        let extraction_chunk = extraction.source_chunk_id.as_str();
        for entity in &extraction.entities {
            absorb_entity(
                &mut node_order,
                &mut nodes,
                document_uuid,
                extraction_chunk,
                entity,
            );
        }
        for relationship in &extraction.relationships {
            absorb_relationship(
                &mut edge_order,
                &mut edges,
                document_uuid,
                extraction_chunk,
                relationship,
            );
        }
    }

    let mut facts = Vec::with_capacity(node_order.len() + edge_order.len());
    let mut contributions = Vec::with_capacity(node_order.len() + edge_order.len());

    for key in &node_order {
        let node = nodes
            .get(key)
            .expect("node order key must exist in accumulator");
        let payload = serde_json::json!({
            "schema": "edgequake.graph.fact.v1",
            "kind": "node",
            "node_id": node.node_id,
            "properties": {
                "entity_type": node.entity_type,
                "description": node.description,
                "importance": node.importance,
                "source_chunk_ids": sorted_strings(&node.source_chunk_ids),
                "source_document_id": node.source_document_id,
                "source_file_path": node.source_file_path,
                "tenant_id": tenant_id,
                "workspace_id": workspace_id,
            }
        });
        push_paired_fact_contribution(
            &mut facts,
            &mut contributions,
            node.fact_id,
            revision,
            &payload,
        )?;
    }

    for key in &edge_order {
        let edge = edges
            .get(key)
            .expect("edge order key must exist in accumulator");
        let payload = serde_json::json!({
            "schema": "edgequake.graph.fact.v1",
            "kind": "edge",
            "source": edge.source,
            "target": edge.target,
            "properties": {
                "relation_type": edge.relation_type,
                "description": edge.description,
                "weight": edge.weight,
                "keywords": sorted_strings(&edge.keywords),
                "source_chunk_ids": sorted_strings(&edge.source_chunk_ids),
                "source_document_id": edge.source_document_id,
                "source_file_path": edge.source_file_path,
                "tenant_id": tenant_id,
                "workspace_id": workspace_id,
            }
        });
        push_paired_fact_contribution(
            &mut facts,
            &mut contributions,
            edge.fact_id,
            revision,
            &payload,
        )?;
    }

    Ok((facts, contributions))
}

/// Emit one fact + one generation-scoped contribution from the same payload.
///
/// Contribution ids include `revision` so reprocess can append without colliding
/// on `graph_contributions.contribution_id` while fact logical ids stay stable.
fn push_paired_fact_contribution(
    facts: &mut Vec<PreparedRecord>,
    contributions: &mut Vec<PreparedRecord>,
    fact_id: Uuid,
    revision: u64,
    payload: &serde_json::Value,
) -> Result<(), StorageError> {
    let fact = prepared_json_value(fact_id, revision, payload)?;
    let mut contribution_payload = payload.clone();
    if let Some(object) = contribution_payload.as_object_mut() {
        object.insert("fact_id".into(), serde_json::json!(fact_id));
        object.insert("fact_revision".into(), serde_json::json!(revision));
    }
    let contribution_id = stable_record_id("contribution", &format!("{fact_id}:{revision}"));
    let contribution = prepared_json_value(contribution_id, revision, &contribution_payload)?;
    facts.push(fact);
    contributions.push(contribution);
    Ok(())
}

#[derive(Debug, Clone)]
struct CanonicalNode {
    fact_id: Uuid,
    node_id: String,
    entity_type: String,
    type_votes: HashMap<String, f64>,
    description: String,
    importance: f32,
    source_chunk_ids: BTreeSet<String>,
    source_document_id: Option<String>,
    source_file_path: Option<String>,
}

#[derive(Debug, Clone)]
struct CanonicalEdge {
    fact_id: Uuid,
    source: String,
    target: String,
    relation_type: String,
    description: String,
    weight: f32,
    keywords: BTreeSet<String>,
    source_chunk_ids: BTreeSet<String>,
    source_document_id: Option<String>,
    source_file_path: Option<String>,
}

fn absorb_entity(
    order: &mut Vec<String>,
    nodes: &mut HashMap<String, CanonicalNode>,
    document_uuid: Uuid,
    extraction_chunk: &str,
    entity: &ExtractedEntity,
) {
    let entity_id = EntityId::new(&entity.name);
    if entity_id.is_empty() {
        return;
    }
    let node_id = entity_id.as_graph_node_id().to_string();
    let key = format!("{document_uuid}:{extraction_chunk}:{node_id}");
    let fact_id = stable_record_id("node", &key);

    if let Some(existing) = nodes.get_mut(&key) {
        merge_entity_type_vote(
            &mut existing.entity_type,
            &mut existing.type_votes,
            &entity.entity_type,
            entity.importance,
            &node_id,
        );
        prefer_longer_description(&mut existing.description, &entity.description);
        existing.importance = merge_importance(existing.importance, entity.importance);
        union_chunk_ids(
            &mut existing.source_chunk_ids,
            extraction_chunk,
            &entity.source_chunk_ids,
        );
        prefer_filled_option(&mut existing.source_document_id, &entity.source_document_id);
        prefer_filled_option(&mut existing.source_file_path, &entity.source_file_path);
        return;
    }

    let (entity_type, type_votes) = seed_entity_type_votes(&entity.entity_type, entity.importance);
    let mut source_chunk_ids = BTreeSet::new();
    union_chunk_ids(
        &mut source_chunk_ids,
        extraction_chunk,
        &entity.source_chunk_ids,
    );
    order.push(key.clone());
    nodes.insert(
        key,
        CanonicalNode {
            fact_id,
            node_id,
            entity_type,
            type_votes,
            description: entity.description.clone(),
            importance: entity.importance,
            source_chunk_ids,
            source_document_id: entity.source_document_id.clone(),
            source_file_path: entity.source_file_path.clone(),
        },
    );
}

fn absorb_relationship(
    order: &mut Vec<String>,
    edges: &mut HashMap<String, CanonicalEdge>,
    document_uuid: Uuid,
    extraction_chunk: &str,
    relationship: &ExtractedRelationship,
) {
    let source_id = EntityId::new(&relationship.source);
    let target_id = EntityId::new(&relationship.target);
    if source_id.is_empty() || target_id.is_empty() {
        return;
    }
    let source = source_id.as_graph_node_id().to_string();
    let target = target_id.as_graph_node_id().to_string();
    if source == target {
        return;
    }
    let relation_type = normalize_relation_type_str(&relationship.relation_type);
    if relation_type.is_empty() {
        return;
    }
    let key = format!("{document_uuid}:{extraction_chunk}:{source}:{target}:{relation_type}");
    let fact_id = stable_record_id("edge", &key);

    if let Some(existing) = edges.get_mut(&key) {
        prefer_longer_description(&mut existing.description, &relationship.description);
        existing.weight = merge_relationship_weight(existing.weight, relationship.weight);
        for kw in &relationship.keywords {
            let trimmed = kw.trim();
            if !trimmed.is_empty() {
                existing.keywords.insert(trimmed.to_string());
            }
        }
        union_chunk_ids(
            &mut existing.source_chunk_ids,
            extraction_chunk,
            &relationship.all_source_chunk_ids(),
        );
        prefer_filled_option(
            &mut existing.source_document_id,
            &relationship.source_document_id,
        );
        prefer_filled_option(
            &mut existing.source_file_path,
            &relationship.source_file_path,
        );
        return;
    }

    let mut keywords = BTreeSet::new();
    for kw in &relationship.keywords {
        let trimmed = kw.trim();
        if !trimmed.is_empty() {
            keywords.insert(trimmed.to_string());
        }
    }
    let mut source_chunk_ids = BTreeSet::new();
    union_chunk_ids(
        &mut source_chunk_ids,
        extraction_chunk,
        &relationship.all_source_chunk_ids(),
    );
    order.push(key.clone());
    edges.insert(
        key,
        CanonicalEdge {
            fact_id,
            source,
            target,
            relation_type,
            description: relationship.description.clone(),
            weight: relationship.weight.max(0.0),
            keywords,
            source_chunk_ids,
            source_document_id: relationship.source_document_id.clone(),
            source_file_path: relationship.source_file_path.clone(),
        },
    );
}

fn sorted_strings(values: &BTreeSet<String>) -> Vec<String> {
    values.iter().cloned().collect()
}

fn prepared_json_record<T>(value: &T, revision: u64) -> Result<PreparedRecord, StorageError>
where
    T: serde::Serialize + HasPreparedIdentity,
{
    let payload = serde_json::to_vec(value)
        .map_err(|error| StorageError::InvalidData(format!("encode prepared record: {error}")))?;
    Ok(PreparedRecord {
        id: value.prepared_id(),
        revision,
        digest: sha256(&payload),
        payload,
    })
}

trait HasPreparedIdentity {
    fn prepared_id(&self) -> Uuid;
}

impl HasPreparedIdentity for Chunk {
    fn prepared_id(&self) -> Uuid {
        self.id.0
    }
}

fn prepared_json_value(
    id: Uuid,
    revision: u64,
    value: &serde_json::Value,
) -> Result<PreparedRecord, StorageError> {
    let payload = serde_json::to_vec(value)
        .map_err(|error| StorageError::InvalidData(format!("encode prepared payload: {error}")))?;
    Ok(PreparedRecord {
        id,
        revision,
        digest: sha256(&payload),
        payload,
    })
}

fn stable_record_id(kind: &str, key: &str) -> Uuid {
    Uuid::new_v5(&Uuid::NAMESPACE_OID, format!("{kind}:{key}").as_bytes())
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn required_scope_uuid(axis: &str, raw: Option<&str>) -> Result<Uuid, StorageError> {
    let raw = raw.ok_or_else(|| {
        StorageError::InvalidData(format!("{axis}_id is required for committed ingestion"))
    })?;
    parse_uuid(raw)
}

/// Insert relational chunks when authority mode and repository are configured.
pub async fn persist_relational_chunks(
    repo: &dyn ChunkRepository,
    ctx: &IngestionPersistContext,
    result: &ProcessingResult,
) -> Result<(), StorageError> {
    let chunks = build_relational_chunks(ctx, result)?;
    if chunks.is_empty() {
        return Ok(());
    }
    repo.insert_batch(&mut UnitOfWork::default(), &chunks)
        .await?;
    Ok(())
}

fn parse_document_id(raw: &str) -> Result<DocumentId, StorageError> {
    // SPEC-118: bare UUID or injection::{ws}::{uuid} → relational DocumentId.
    resolve_relational_document_id(raw)
}

fn parse_uuid(raw: &str) -> Result<Uuid, StorageError> {
    Uuid::parse_str(raw)
        .map_err(|e| StorageError::InvalidData(format!("invalid uuid '{raw}': {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunker::TextChunk;
    use crate::extractor::ExtractionResult;
    use edgequake_storage::MemoryChunkRepository;
    use std::collections::HashSet;
    use std::sync::Arc;

    fn sample_result(doc_uuid: &str) -> ProcessingResult {
        ProcessingResult {
            document_id: doc_uuid.into(),
            chunks: vec![TextChunk {
                id: format!("{doc_uuid}-chunk-0"),
                content: "relational text".into(),
                index: 0,
                start_offset: 0,
                end_offset: 15,
                start_line: 1,
                end_line: 1,
                token_count: 3,
                embedding: None,
                section: None,
                page_start: None,
                page_end: None,
                modality: None,
            }],
            extractions: vec![],
            stats: Default::default(),
            lineage: None,
        }
    }

    fn scoped_ctx(doc_id: Uuid) -> (IngestionPersistContext, Uuid, Uuid) {
        let tenant = Uuid::new_v4();
        let workspace = Uuid::new_v4();
        let ctx = IngestionPersistContext::new(
            doc_id.to_string(),
            Some(tenant.to_string()),
            Some(workspace.to_string()),
        );
        (ctx, tenant, workspace)
    }

    fn entity(
        name: &str,
        entity_type: &str,
        description: &str,
        importance: f32,
    ) -> ExtractedEntity {
        let mut entity = ExtractedEntity::new(name, entity_type, description);
        entity.importance = importance;
        entity
    }

    fn relationship(
        source: &str,
        target: &str,
        relation_type: &str,
        description: &str,
        weight: f32,
        keywords: &[&str],
    ) -> ExtractedRelationship {
        let mut rel = ExtractedRelationship::new(source, target, relation_type);
        rel.description = description.into();
        rel.weight = weight;
        rel.keywords = keywords.iter().map(|k| (*k).to_string()).collect();
        rel
    }

    #[tokio::test]
    async fn contract_spec091_single_chunk_writer() {
        let doc_id = Uuid::new_v4();
        let ctx = IngestionPersistContext::new(doc_id.to_string(), None, None);
        let repo = Arc::new(MemoryChunkRepository::new());
        persist_relational_chunks(repo.as_ref(), &ctx, &sample_result(&doc_id.to_string()))
            .await
            .expect("relational insert");

        let page = repo.scan_from(None, 10).await.expect("scan");
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].content, "relational text");
        assert_eq!(page.items[0].document_id.into_uuid(), doc_id);
    }

    #[test]
    fn contract_spec091_build_relational_chunks_rejects_bad_document_id() {
        let ctx = IngestionPersistContext::new("not-a-uuid", None, None);
        let err = build_relational_chunks(&ctx, &sample_result("not-a-uuid")).unwrap_err();
        assert!(matches!(err, StorageError::InvalidData(_)));
    }

    #[test]
    fn contract_spec118_build_chunks_maps_injection_doc_id() {
        let ws = Uuid::new_v4();
        let inj = Uuid::new_v4();
        let composite = format!("injection::{ws}::{inj}");
        let ctx = IngestionPersistContext::new(composite.clone(), None, None);
        let chunks = build_relational_chunks(&ctx, &sample_result(&composite)).expect("map");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].document_id.into_uuid(), inj);
        assert_eq!(
            chunks[0]
                .metadata
                .get("legacy_document_id")
                .and_then(|v| v.as_str()),
            Some(composite.as_str())
        );
    }

    #[tokio::test]
    async fn contract_spec118_persist_injection_composite_document_id() {
        let ws = Uuid::new_v4();
        let inj = Uuid::new_v4();
        let composite = format!("injection::{ws}::{inj}");
        let ctx = IngestionPersistContext::new(composite.clone(), None, None);
        let repo = Arc::new(MemoryChunkRepository::new());
        persist_relational_chunks(repo.as_ref(), &ctx, &sample_result(&composite))
            .await
            .expect("injection relational insert");

        let page = repo.scan_from(None, 10).await.expect("scan");
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].document_id.into_uuid(), inj);
    }

    #[test]
    fn canonical_batch_collapses_duplicate_normalized_entities() {
        let doc_id = Uuid::new_v4();
        let (ctx, _, _) = scoped_ctx(doc_id);
        let chunk_id = format!("{doc_id}-chunk-0");
        let mut result = sample_result(&doc_id.to_string());
        let mut extraction = ExtractionResult::new(&chunk_id);
        extraction.entities = vec![
            entity("Alpha Beta", "PERSON", "short", 0.4),
            entity(
                "alpha beta",
                "ORGANIZATION",
                "a much longer description for quality",
                0.9,
            ),
            entity("", "PERSON", "empty name skipped", 1.0),
        ];
        result.extractions = vec![extraction];
        let chunks = build_relational_chunks(&ctx, &result).expect("chunks");
        let batch =
            build_prepared_ingestion_batch(&ctx, &result, &chunks, "text-embedding-3-small", 1)
                .expect("batch");

        assert_eq!(batch.facts.len(), 1);
        assert_eq!(batch.contributions.len(), 1);
        assert_ne!(batch.facts[0].id, batch.contributions[0].id);
        assert_eq!(batch.facts[0].revision, 1);
        assert_eq!(batch.contributions[0].revision, 1);
        let payload: serde_json::Value =
            serde_json::from_slice(&batch.facts[0].payload).expect("json");
        assert_eq!(payload["node_id"], "ALPHA_BETA");
        assert_eq!(
            payload["properties"]["description"],
            "a much longer description for quality"
        );
        assert!((payload["properties"]["importance"].as_f64().unwrap() - 0.9).abs() < 1e-6);
        assert_eq!(payload["properties"]["entity_type"], "ORGANIZATION");
        let contribution: serde_json::Value =
            serde_json::from_slice(&batch.contributions[0].payload).expect("json");
        assert_eq!(
            contribution["fact_id"]
                .as_str()
                .and_then(|s| Uuid::parse_str(s).ok()),
            Some(batch.facts[0].id)
        );
    }

    #[test]
    fn canonical_batch_collapses_case_variant_relationships_and_skips_self_edges() {
        let doc_id = Uuid::new_v4();
        let (ctx, _, _) = scoped_ctx(doc_id);
        let chunk_id = format!("{doc_id}-chunk-0");
        let mut result = sample_result(&doc_id.to_string());
        let mut extraction = ExtractionResult::new(&chunk_id);
        extraction.entities = vec![
            entity("Alice", "PERSON", "person a", 0.5),
            entity("Bob", "PERSON", "person b", 0.5),
        ];
        extraction.relationships = vec![
            relationship("Alice", "Bob", "knows", "short", 0.2, &["b", "a"]),
            relationship(
                "alice",
                "bob",
                "KNOWS",
                "much longer relationship description",
                0.8,
                &["c", "a"],
            ),
            relationship("Alice", "Alice", "self", "should skip", 1.0, &[]),
        ];
        result.extractions = vec![extraction];
        let chunks = build_relational_chunks(&ctx, &result).expect("chunks");
        let batch =
            build_prepared_ingestion_batch(&ctx, &result, &chunks, "text-embedding-3-small", 2)
                .expect("batch");

        assert_eq!(batch.facts.len(), 3); // 2 nodes + 1 edge
        assert_eq!(batch.contributions.len(), 3);
        assert!(batch
            .facts
            .iter()
            .chain(&batch.contributions)
            .all(|record| record.revision == 2));
        let edge = batch
            .facts
            .iter()
            .find(|record| {
                let payload: serde_json::Value =
                    serde_json::from_slice(&record.payload).expect("json");
                payload["kind"] == "edge"
            })
            .expect("edge fact");
        let payload: serde_json::Value = serde_json::from_slice(&edge.payload).expect("json");
        assert_eq!(payload["properties"]["relation_type"], "KNOWS");
        assert_eq!(
            payload["properties"]["description"],
            "much longer relationship description"
        );
        assert!((payload["properties"]["weight"].as_f64().unwrap() - 0.8).abs() < 1e-6);
        let keywords = payload["properties"]["keywords"]
            .as_array()
            .expect("keywords");
        assert_eq!(
            keywords
                .iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b", "c"]
        );
    }

    #[test]
    fn canonical_batch_merges_duplicate_extraction_results_for_same_chunk() {
        let doc_id = Uuid::new_v4();
        let (ctx, _, _) = scoped_ctx(doc_id);
        let chunk_id = format!("{doc_id}-chunk-0");
        let mut result = sample_result(&doc_id.to_string());
        let mut first = ExtractionResult::new(&chunk_id);
        first.entities =
            vec![entity("Acme", "ORGANIZATION", "corp", 0.5).with_source_chunk_id("extra-a")];
        let mut second = ExtractionResult::new(&chunk_id);
        second.entities = vec![entity("ACME", "ORGANIZATION", "corporation longer", 0.7)
            .with_source_chunk_id("extra-b")];
        result.extractions = vec![first, second];
        let chunks = build_relational_chunks(&ctx, &result).expect("chunks");
        let batch =
            build_prepared_ingestion_batch(&ctx, &result, &chunks, "text-embedding-3-small", 1)
                .expect("batch");
        assert_eq!(batch.facts.len(), 1);
        let payload: serde_json::Value =
            serde_json::from_slice(&batch.facts[0].payload).expect("json");
        let provenance = payload["properties"]["source_chunk_ids"]
            .as_array()
            .expect("provenance")
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>();
        assert!(provenance.contains(&chunk_id.as_str()));
        assert!(provenance.contains(&"extra-a"));
        assert!(provenance.contains(&"extra-b"));
        // Sorted for digest stability.
        let mut sorted = provenance.clone();
        sorted.sort();
        assert_eq!(provenance, sorted);
    }

    #[test]
    fn canonical_batch_is_digest_stable_across_duplicate_mention_order() {
        let doc_id = Uuid::new_v4();
        let (ctx, _, _) = scoped_ctx(doc_id);
        let chunk_id = format!("{doc_id}-chunk-0");
        let mut left = sample_result(&doc_id.to_string());
        let mut left_extraction = ExtractionResult::new(&chunk_id);
        left_extraction.entities = vec![
            entity("Zed", "PERSON", "z", 0.1),
            entity("zed", "PERSON", "zed longer", 0.2),
            entity("Amy", "PERSON", "a", 0.3),
        ];
        left.extractions = vec![left_extraction];

        let mut right = sample_result(&doc_id.to_string());
        let mut right_extraction = ExtractionResult::new(&chunk_id);
        // Same first-seen key order for distinct entities; duplicates collapse.
        right_extraction.entities = vec![
            entity("Zed", "PERSON", "z", 0.1),
            entity("Amy", "PERSON", "a", 0.3),
            entity("zed", "PERSON", "zed longer", 0.2),
        ];
        right.extractions = vec![right_extraction];

        let left_chunks = build_relational_chunks(&ctx, &left).expect("chunks");
        let right_chunks = build_relational_chunks(&ctx, &right).expect("chunks");
        let left_batch =
            build_prepared_ingestion_batch(&ctx, &left, &left_chunks, "text-embedding-3-small", 1)
                .expect("left");
        let right_batch = build_prepared_ingestion_batch(
            &ctx,
            &right,
            &right_chunks,
            "text-embedding-3-small",
            1,
        )
        .expect("right");
        assert_eq!(left_batch.canonical_digest, right_batch.canonical_digest);
        assert_eq!(left_batch.facts.len(), 2);
        assert_eq!(right_batch.facts.len(), 2);
    }

    #[test]
    fn canonical_batch_fact_and_contribution_ids_stay_paired_and_unique() {
        let doc_id = Uuid::new_v4();
        let (ctx, _, _) = scoped_ctx(doc_id);
        let chunk_id = format!("{doc_id}-chunk-0");
        let mut result = sample_result(&doc_id.to_string());
        let mut extraction = ExtractionResult::new(&chunk_id);
        extraction.entities = vec![
            entity("One", "CONCEPT", "1", 0.5),
            entity("one", "CONCEPT", "one longer", 0.6),
            entity("Two", "CONCEPT", "2", 0.5),
        ];
        extraction.relationships = vec![relationship("One", "Two", "related", "link", 0.5, &[])];
        result.extractions = vec![extraction];
        let chunks = build_relational_chunks(&ctx, &result).expect("chunks");
        let batch =
            build_prepared_ingestion_batch(&ctx, &result, &chunks, "text-embedding-3-small", 3)
                .expect("batch");

        assert_eq!(batch.facts.len(), batch.contributions.len());
        assert_eq!(batch.ingest_generation, 3);
        assert_eq!(batch.expected_revision, Some(2));
        assert_eq!(batch.idempotency_key, format!("{}:3:0", doc_id));
        let fact_keys: HashSet<_> = batch.facts.iter().map(|r| (r.id, r.revision)).collect();
        assert_eq!(fact_keys.len(), batch.facts.len());
        let contribution_keys: HashSet<_> = batch
            .contributions
            .iter()
            .map(|r| (r.id, r.revision))
            .collect();
        assert_eq!(contribution_keys.len(), batch.contributions.len());
        for (fact, contribution) in batch.facts.iter().zip(&batch.contributions) {
            assert_ne!(fact.id, contribution.id);
            assert_eq!(fact.revision, contribution.revision);
            let payload: serde_json::Value =
                serde_json::from_slice(&contribution.payload).expect("json");
            assert_eq!(
                payload["fact_id"]
                    .as_str()
                    .and_then(|s| Uuid::parse_str(s).ok()),
                Some(fact.id)
            );
        }
    }

    /// Dense synthetic doc: two chunks each with 3k entities → >10k total records.
    fn dense_two_chunk_result(doc_id: Uuid, entities_per_chunk: usize) -> ProcessingResult {
        let doc = doc_id.to_string();
        let mut chunks = Vec::with_capacity(2);
        let mut extractions = Vec::with_capacity(2);
        for index in 0..2 {
            let chunk_id = format!("{doc}-chunk-{index}");
            chunks.push(TextChunk {
                id: chunk_id.clone(),
                content: format!("chunk {index}"),
                index,
                start_offset: index * 10,
                end_offset: index * 10 + 9,
                start_line: 1,
                end_line: 1,
                token_count: 2,
                embedding: Some(vec![0.1, 0.2]),
                section: None,
                page_start: None,
                page_end: None,
                modality: None,
            });
            let mut extraction = ExtractionResult::new(&chunk_id);
            extraction.entities = (0..entities_per_chunk)
                .map(|n| {
                    entity(
                        &format!("Entity{index}_{n}"),
                        "CONCEPT",
                        &format!("desc {index} {n}"),
                        0.5,
                    )
                })
                .collect();
            extractions.push(extraction);
        }
        ProcessingResult {
            document_id: doc,
            chunks,
            extractions,
            stats: Default::default(),
            lineage: None,
        }
    }

    #[test]
    fn pack_splits_when_total_exceeds_max_batch_records() {
        let doc_id = Uuid::new_v4();
        let (ctx, _, _) = scoped_ctx(doc_id);
        // 3000 entities → 1 chunk + 3000 facts + 3000 contribs + 1 emb = 6002 per chunk.
        // Two chunks = 12004 > 10000 → must split into 2 packs.
        let result = dense_two_chunk_result(doc_id, 3000);
        let chunks = build_relational_chunks(&ctx, &result).expect("chunks");
        let single =
            build_prepared_ingestion_batch(&ctx, &result, &chunks, "text-embedding-3-small", 5)
                .expect("single batch builds before admission");
        let single_total = single.chunks.len()
            + single.facts.len()
            + single.contributions.len()
            + single.embeddings.len();
        assert!(
            single_total > MAX_BATCH_RECORDS,
            "fixture must exceed admission cap (got {single_total})"
        );

        let batches =
            pack_prepared_ingestion_batches(&ctx, &result, &chunks, "text-embedding-3-small", 5)
                .expect("pack");
        assert!(
            batches.len() >= 2,
            "expected multi-batch, got {}",
            batches.len()
        );
        assert_eq!(batches[0].batch_ordinal, 0);
        assert_eq!(batches[0].expected_revision, Some(4));
        assert_eq!(batches[1].batch_ordinal, 1);
        assert_eq!(batches[1].expected_revision, Some(5));
        assert_eq!(batches[0].idempotency_key, format!("{}:5:0", doc_id));
        assert_eq!(batches[1].idempotency_key, format!("{}:5:1", doc_id));
        for batch in &batches {
            let total = batch.chunks.len()
                + batch.facts.len()
                + batch.contributions.len()
                + batch.embeddings.len();
            assert!(
                total <= MAX_BATCH_RECORDS,
                "batch {} has {total} records",
                batch.batch_ordinal
            );
            assert!(edgequake_storage_contracts::validate_prepared_ingestion_batch(batch).is_ok());
        }
    }

    #[test]
    fn pack_chunk_index_ranges_respects_budget() {
        assert_eq!(
            pack_chunk_index_ranges(&[100, 100, 100], 250),
            vec![(0, 2), (2, 3)]
        );
        assert_eq!(pack_chunk_index_ranges(&[50, 50], 200), vec![(0, 2)]);
        assert_eq!(pack_chunk_index_ranges(&[], 100), vec![(0, 0)]);
    }
}

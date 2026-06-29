//! SPEC-024 2.5 — chunk text SSOT in KV; vector metadata references chunk id only.

use serde_json::{json, Value};

use crate::chunker::TextChunk;
use crate::pipeline::ProcessingResult;

use super::persistence::{ChunkVectorBuildOptions, IngestionPersistContext};

/// Build KV records for chunk text (authoritative content store).
pub fn build_chunk_kv_records(
    document_id: &str,
    source_file: Option<&str>,
    result: &ProcessingResult,
) -> Vec<(String, Value)> {
    result
        .chunks
        .iter()
        .map(|c| (c.id.clone(), chunk_kv_value(document_id, source_file, c)))
        .collect()
}

fn chunk_kv_value(document_id: &str, source_file: Option<&str>, chunk: &TextChunk) -> Value {
    let mut value = json!({
        "content": chunk.content,
        "document_id": document_id,
        "index": chunk.index,
        "start_line": chunk.start_line,
        "end_line": chunk.end_line,
        "start_offset": chunk.start_offset,
        "end_offset": chunk.end_offset,
        "token_count": chunk.token_count,
    });
    if let Some(file) = source_file {
        value["source_file"] = json!(file);
    }
    if let Some(section) = &chunk.section {
        value["section"] = json!({
            "heading_path": section.heading_path,
            "heading_level": section.heading_level,
        });
    }
    // SPEC-032 W-09: page attribution for PDF sources
    if let Some(page) = chunk.page_start {
        value["page_start"] = json!(page);
        value["page_end"] = json!(chunk.page_end.unwrap_or(page));
    }
    value
}

/// Vector metadata for a chunk embedding row (no inline content — use KV + `content_ref`).
pub fn build_chunk_vector_metadata(
    chunk: &TextChunk,
    ctx: &IngestionPersistContext,
    options: ChunkVectorBuildOptions,
) -> Value {
    let mut metadata = json!({
        "type": "chunk",
        "document_id": ctx.document_id,
        "index": chunk.index,
        "content_ref": chunk.id,
    });

    if options.include_lineage_metadata {
        metadata["start_line"] = json!(chunk.start_line);
        metadata["end_line"] = json!(chunk.end_line);
        metadata["start_offset"] = json!(chunk.start_offset);
        metadata["end_offset"] = json!(chunk.end_offset);
        metadata["token_count"] = json!(chunk.token_count);
        // SPEC-032 W-09: page attribution — enables deep-link citations to PDF pages
        if let Some(page) = chunk.page_start {
            metadata["page_start"] = json!(page);
            metadata["page_end"] = json!(chunk.page_end.unwrap_or(page));
        }
    }

    if let Some(tenant_id) = &ctx.tenant_id {
        metadata["tenant_id"] = json!(tenant_id);
    }
    if let Some(workspace_id) = &ctx.workspace_id {
        metadata["workspace_id"] = json!(workspace_id);
    }
    if let Some(source_type) = &ctx.source_type {
        metadata["source_type"] = json!(source_type);
        metadata["source"] = json!(source_type);
    }
    if let Some(source_file_path) = &ctx.source_file_path {
        metadata["source_file_path"] = json!(source_file_path);
    }
    metadata["source_document_id"] = json!(ctx.document_id);

    metadata
}

/// Estimate metadata JSON byte size for chunk vector rows (contract metric).
pub fn chunk_vector_metadata_json_len(metadata: &Value) -> usize {
    metadata.to_string().len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunker::TextChunk;

    fn sample_chunk() -> TextChunk {
        TextChunk {
            id: "doc-chunk-0".into(),
            content: "A".repeat(2_000),
            index: 0,
            start_offset: 0,
            end_offset: 2000,
            start_line: 1,
            end_line: 40,
            token_count: 500,
            embedding: Some(vec![0.0; 8]),
            section: None,
            page_start: None,
            page_end: None,
        }
    }

    #[test]
    fn contract_vector_metadata_omits_inline_content() {
        let chunk = sample_chunk();
        let ctx = IngestionPersistContext::new("doc", None, Some("ws".into()));
        let meta = build_chunk_vector_metadata(&chunk, &ctx, ChunkVectorBuildOptions::STANDARD);
        assert!(meta.get("content").is_none());
        assert_eq!(
            meta.get("content_ref").and_then(|v| v.as_str()),
            Some("doc-chunk-0")
        );

        let legacy = json!({
            "type": "chunk",
            "document_id": "doc",
            "index": 0,
            "content": chunk.content,
        });
        let new_len = chunk_vector_metadata_json_len(&meta);
        let old_len = chunk_vector_metadata_json_len(&legacy);
        assert!(
            new_len * 2 <= old_len,
            "deduped metadata should be at least 50% smaller: new={new_len} old={old_len}"
        );
    }
}

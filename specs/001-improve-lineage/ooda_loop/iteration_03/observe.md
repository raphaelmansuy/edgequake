# Observation - Iteration 03

## Files Examined

- `edgequake/crates/edgequake-core/src/types/document.rs` — Document struct missing type-safe lineage fields

## Current State

- Document uses generic `metadata: Option<serde_json::Value>` for extra data
- No explicit `document_type`, `file_size`, `sha256_checksum`, `pdf_id`, `llm_model`, `embedding_model`, `processed_at`

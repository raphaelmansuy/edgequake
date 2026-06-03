# PDF Metadata Enrichment Pipeline — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a fast async pre-pipeline that extracts text from the first 5 pages of a PDF with EdgeParser, sends it to a local OpenAI-compatible VLM, and stores `summary / topic / language / keywords` in document metadata before the full Graph-RAG pipeline runs.

**Architecture:** Two separate Tokio worker pools — an existing main pool for full pipeline tasks and a new enrichment pool (4 workers by default). Both share the same `task_storage`. The enrichment task is enqueued simultaneously with `PdfProcessing` at upload time; because enrichment is fast (seconds) it finishes long before full indexing completes. A pre-generated `document_id` ties both tasks to the same metadata record.

**Tech Stack:** Rust, `edgequake-tasks` (task/worker infrastructure), `edgequake-pdf` (EdgeParsePdfConverter), `reqwest` (VLM HTTP calls), PostgreSQL KV storage.

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `edgequake/crates/edgequake-tasks/src/types/status.rs` | Add `TaskType::MetadataEnrich` |
| Modify | `edgequake/crates/edgequake-tasks/src/types/data.rs` | Add `MetadataEnrichData` payload |
| Modify | `edgequake/crates/edgequake-tasks/src/lib.rs` | Re-export `MetadataEnrichData` |
| Create | `edgequake/crates/edgequake-api/src/processor/enrichment_config.rs` | Read env vars into `EnrichmentConfig` |
| Create | `edgequake/crates/edgequake-api/src/processor/metadata_enrich.rs` | `MetadataEnrichProcessor` impl |
| Modify | `edgequake/crates/edgequake-api/src/processor/mod.rs` | Declare new sub-modules |
| Modify | `edgequake/crates/edgequake-api/src/state/mod.rs` | Add `enrich_queue: SharedTaskQueue` field |
| Modify | `edgequake/crates/edgequake-api/src/state/postgres.rs` | Initialize `enrich_queue` |
| Modify | `edgequake/crates/edgequake-api/src/state/memory.rs` | Initialize `enrich_queue` |
| Modify | `edgequake/crates/edgequake-api/src/handlers/pdf_upload/helpers.rs` | Pre-generate `document_id`, enqueue enrichment task |
| Modify | `edgequake/src/main.rs` | Start enrichment worker pool |

---

## Task 1: Add `TaskType::MetadataEnrich` and `MetadataEnrichData`

**Files:**
- Modify: `edgequake/crates/edgequake-tasks/src/types/status.rs`
- Modify: `edgequake/crates/edgequake-tasks/src/types/data.rs`
- Modify: `edgequake/crates/edgequake-tasks/src/lib.rs`

- [ ] **Step 1: Write failing test for new task type**

Add to the end of `edgequake/crates/edgequake-tasks/src/types/mod.rs` inside the existing `#[cfg(test)] mod tests { ... }` block (before the closing `}`):

```rust
#[test]
fn test_metadata_enrich_task_type() {
    let task = Task::new(
        test_tenant_id(),
        test_workspace_id(),
        TaskType::MetadataEnrich,
        serde_json::json!({}),
    );
    assert_eq!(task.task_type, TaskType::MetadataEnrich);
    assert!(task.track_id.starts_with("metadata_enrich-"));
}

#[test]
fn test_metadata_enrich_data_serialization() {
    use crate::types::MetadataEnrichData;
    let data = MetadataEnrichData {
        document_id: "doc-123".to_string(),
        pdf_id: uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        tenant_id: test_tenant_id(),
        workspace_id: test_workspace_id(),
        max_pages: 5,
    };
    let json = serde_json::to_value(&data).unwrap();
    let back: MetadataEnrichData = serde_json::from_value(json).unwrap();
    assert_eq!(back.document_id, "doc-123");
    assert_eq!(back.max_pages, 5);
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd /home/romy/workspace/edgequake-romy/edgequake
cargo test -p edgequake-tasks test_metadata_enrich 2>&1 | tail -20
```

Expected: compile error — `TaskType::MetadataEnrich` and `MetadataEnrichData` don't exist yet.

- [ ] **Step 3: Add `MetadataEnrich` to `TaskType` enum**

In `edgequake/crates/edgequake-tasks/src/types/status.rs`, add the variant and its display/track-id:

```rust
/// Task type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    Upload,
    Insert,
    Scan,
    Reindex,
    PdfProcessing,
    MetadataEnrich,
}

impl fmt::Display for TaskType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Upload => write!(f, "upload"),
            Self::Insert => write!(f, "insert"),
            Self::Scan => write!(f, "scan"),
            Self::Reindex => write!(f, "reindex"),
            Self::PdfProcessing => write!(f, "pdf_processing"),
            Self::MetadataEnrich => write!(f, "metadata_enrich"),
        }
    }
}
```

`generate_track_id` in `task.rs` uses `format!("{}-{}", task_type, uuid)` — it calls `Display` automatically, so no changes needed in that function. The new variant's track IDs will be `metadata_enrich-<uuid>` as soon as the `Display` impl is in place.

- [ ] **Step 4: Add `MetadataEnrichData` to `data.rs`**

Append to `edgequake/crates/edgequake-tasks/src/types/data.rs`:

```rust
/// Metadata enrichment task payload.
///
/// Carries everything the enrichment worker needs to extract a summary,
/// topic, language, and keywords from the first `max_pages` of a PDF.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataEnrichData {
    /// Pre-generated document ID shared with the PdfProcessing task.
    /// The enrichment worker writes results to `{document_id}-metadata`.
    pub document_id: String,

    /// PDF identifier in pdf_storage (used to load raw bytes).
    pub pdf_id: uuid::Uuid,

    /// Tenant ID for multi-tenant isolation.
    pub tenant_id: uuid::Uuid,

    /// Workspace ID for isolation.
    pub workspace_id: uuid::Uuid,

    /// Maximum number of pages to extract text from (default 5).
    #[serde(default = "default_max_pages")]
    pub max_pages: usize,
}

fn default_max_pages() -> usize {
    5
}
```

- [ ] **Step 5: Re-export `MetadataEnrichData` from `lib.rs`**

In `edgequake/crates/edgequake-tasks/src/lib.rs`, find the existing `pub use types::{ ... }` block and add `MetadataEnrichData` to it:

```rust
pub use types::{
    ChunkProgress, DirectoryScanData, DocumentUploadData, MetadataEnrichData, PdfProcessingData,
    ReindexData, Task, TaskFailureInfo, TaskProgress, TaskStatus, TaskType, TextInsertData,
};
```

- [ ] **Step 6: Run test to verify it passes**

```bash
cd /home/romy/workspace/edgequake-romy/edgequake
cargo test -p edgequake-tasks test_metadata_enrich 2>&1 | tail -20
```

Expected: both tests PASS.

- [ ] **Step 7: Commit**

```bash
cd /home/romy/workspace/edgequake-romy/edgequake
git add crates/edgequake-tasks/src/types/status.rs \
        crates/edgequake-tasks/src/types/data.rs \
        crates/edgequake-tasks/src/types/task.rs \
        crates/edgequake-tasks/src/types/mod.rs \
        crates/edgequake-tasks/src/lib.rs
git commit -m "feat(tasks): add MetadataEnrich task type and MetadataEnrichData payload"
```

---

## Task 2: `EnrichmentConfig`

**Files:**
- Create: `edgequake/crates/edgequake-api/src/processor/enrichment_config.rs`

- [ ] **Step 1: Write failing test**

Create `edgequake/crates/edgequake-api/src/processor/enrichment_config.rs` with only the test first:

```rust
#[derive(Debug, Clone)]
pub struct EnrichmentConfig {
    pub vlm_base_url: String,
    pub vlm_model: String,
    pub max_pages: usize,
    pub concurrent: usize,
}

impl EnrichmentConfig {
    pub fn from_env() -> Self {
        Self {
            vlm_base_url: std::env::var("ENRICHMENT_VLM_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:11434/v1".to_string()),
            vlm_model: std::env::var("ENRICHMENT_VLM_MODEL")
                .unwrap_or_else(|_| "llava:7b".to_string()),
            max_pages: std::env::var("ENRICHMENT_MAX_PAGES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            concurrent: std::env::var("ENRICHMENT_CONCURRENT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(4),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enrichment_config_defaults() {
        // Ensure no stale env vars from other tests
        std::env::remove_var("ENRICHMENT_VLM_BASE_URL");
        std::env::remove_var("ENRICHMENT_VLM_MODEL");
        std::env::remove_var("ENRICHMENT_MAX_PAGES");
        std::env::remove_var("ENRICHMENT_CONCURRENT");

        let config = EnrichmentConfig::from_env();

        assert_eq!(config.vlm_base_url, "http://localhost:11434/v1");
        assert_eq!(config.vlm_model, "llava:7b");
        assert_eq!(config.max_pages, 5);
        assert_eq!(config.concurrent, 4);
    }

    #[test]
    fn test_enrichment_config_from_env() {
        std::env::set_var("ENRICHMENT_VLM_BASE_URL", "http://myhost:8080/v1");
        std::env::set_var("ENRICHMENT_VLM_MODEL", "gemma3:12b");
        std::env::set_var("ENRICHMENT_MAX_PAGES", "3");
        std::env::set_var("ENRICHMENT_CONCURRENT", "8");

        let config = EnrichmentConfig::from_env();

        // Restore
        std::env::remove_var("ENRICHMENT_VLM_BASE_URL");
        std::env::remove_var("ENRICHMENT_VLM_MODEL");
        std::env::remove_var("ENRICHMENT_MAX_PAGES");
        std::env::remove_var("ENRICHMENT_CONCURRENT");

        assert_eq!(config.vlm_base_url, "http://myhost:8080/v1");
        assert_eq!(config.vlm_model, "gemma3:12b");
        assert_eq!(config.max_pages, 3);
        assert_eq!(config.concurrent, 8);
    }
}
```

- [ ] **Step 2: Declare module in `processor/mod.rs`**

In `edgequake/crates/edgequake-api/src/processor/mod.rs`, add after the existing `mod` declarations (e.g. after `mod workspace_resolver;`):

```rust
pub mod enrichment_config;
pub mod metadata_enrich;
```

- [ ] **Step 3: Run test**

```bash
cd /home/romy/workspace/edgequake-romy/edgequake
cargo test -p edgequake-api test_enrichment_config 2>&1 | tail -20
```

Expected: both config tests PASS (the `metadata_enrich` module declaration will cause a compile error until Task 3 — add an empty file first if needed):

```bash
touch crates/edgequake-api/src/processor/metadata_enrich.rs
```

Then re-run.

- [ ] **Step 4: Commit**

```bash
cd /home/romy/workspace/edgequake-romy/edgequake
git add crates/edgequake-api/src/processor/enrichment_config.rs \
        crates/edgequake-api/src/processor/mod.rs \
        crates/edgequake-api/src/processor/metadata_enrich.rs
git commit -m "feat(api): add EnrichmentConfig reading from env vars"
```

---

## Task 3: `MetadataEnrichProcessor`

**Files:**
- Modify: `edgequake/crates/edgequake-api/src/processor/metadata_enrich.rs`

This processor implements `TaskProcessor`. It loads PDF bytes from `pdf_storage`, runs EdgeParser, truncates text to 8000 chars, calls the local VLM, and writes results to `{document_id}-metadata` in KV storage.

- [ ] **Step 1: Write the full processor**

Replace the contents of `edgequake/crates/edgequake-api/src/processor/metadata_enrich.rs`:

```rust
use async_trait::async_trait;
use edgequake_pdf::{backend::edgeparse::EdgeParsePdfConverter, PdfConversionConfig, PdfConverter};
use edgequake_storage::{traits::KVStorage, PdfDocumentStorage};
use edgequake_tasks::{MetadataEnrichData, Task, TaskError, TaskProcessor, TaskResult};
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use super::enrichment_config::EnrichmentConfig;

/// Maximum characters sent to the VLM (~8000 chars ≈ first 5 PDF pages of text).
const MAX_TEXT_CHARS: usize = 8_000;

const ENRICHMENT_PROMPT: &str = "You are a document analyst. Given the text from the first pages \
of a document, extract structured metadata. Respond ONLY with valid JSON, no markdown, \
no explanation:\n\
{\n\
  \"summary\": \"2-3 paragraph summary written in the document's own language\",\n\
  \"topic\": \"single short topic phrase\",\n\
  \"language\": \"ISO 639-1 code (e.g. en, id, fr)\",\n\
  \"keywords\": [\"up to 10 keywords\"]\n\
}\n\nDocument text:";

#[derive(Debug, Deserialize)]
struct EnrichmentResponse {
    summary: String,
    topic: String,
    language: String,
    keywords: Vec<String>,
}

pub struct MetadataEnrichProcessor {
    kv_storage: Arc<dyn KVStorage>,
    pdf_storage: Option<Arc<dyn PdfDocumentStorage>>,
    config: EnrichmentConfig,
    http_client: Client,
}

impl MetadataEnrichProcessor {
    pub fn new(
        kv_storage: Arc<dyn KVStorage>,
        pdf_storage: Option<Arc<dyn PdfDocumentStorage>>,
        config: EnrichmentConfig,
    ) -> Self {
        Self {
            kv_storage,
            pdf_storage,
            config,
            http_client: Client::new(),
        }
    }

    async fn load_pdf_bytes(&self, pdf_id: &uuid::Uuid) -> TaskResult<Vec<u8>> {
        let storage = self.pdf_storage.as_ref().ok_or_else(|| {
            TaskError::Processing("pdf_storage not available (postgres feature required)".to_string())
        })?;

        let doc = storage
            .get_pdf(pdf_id)
            .await
            .map_err(|e| TaskError::Processing(format!("Failed to load PDF: {}", e)))?
            .ok_or_else(|| TaskError::NotFound(format!("PDF not found: {}", pdf_id)))?;

        Ok(doc.pdf_data)
    }

    async fn write_metadata(&self, document_id: &str, updates: serde_json::Value) -> TaskResult<()> {
        let key = format!("{}-metadata", document_id);

        // Merge updates into existing metadata (create if absent)
        let existing = self
            .kv_storage
            .get_by_id(&key)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| serde_json::json!({}));

        let mut obj = existing
            .as_object()
            .cloned()
            .unwrap_or_default();

        if let Some(map) = updates.as_object() {
            for (k, v) in map {
                obj.insert(k.clone(), v.clone());
            }
        }

        self.kv_storage
            .upsert(&[(key, serde_json::Value::Object(obj))])
            .await
            .map_err(|e| TaskError::Processing(format!("KV write failed: {}", e)))
    }

    async fn call_vlm(&self, text: &str) -> Result<EnrichmentResponse, String> {
        let prompt = format!("{}\n\n{}", ENRICHMENT_PROMPT, text);

        let body = serde_json::json!({
            "model": self.config.vlm_model,
            "messages": [{"role": "user", "content": prompt}],
            "temperature": 0.1,
        });

        let url = format!("{}/chat/completions", self.config.vlm_base_url);

        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| format!("VLM request failed: {}", e))?;

        let resp_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to read VLM response body: {}", e))?;

        let content = resp_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| format!("Unexpected VLM response shape: {}", resp_json))?;

        // Strip optional ```json ... ``` fences that some models add
        let json_str = content
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        serde_json::from_str::<EnrichmentResponse>(json_str)
            .map_err(|e| format!("VLM returned invalid JSON ({e}): {json_str}"))
    }
}

#[async_trait]
impl TaskProcessor for MetadataEnrichProcessor {
    async fn process(
        &self,
        task: &mut Task,
        _cancel_token: CancellationToken,
    ) -> TaskResult<serde_json::Value> {
        let data: MetadataEnrichData =
            serde_json::from_value(task.task_data.clone()).map_err(|e| {
                TaskError::InvalidPayload(format!("Invalid MetadataEnrichData: {}", e))
            })?;

        // Mark enrichment as in-progress so polling clients see the transition
        self.write_metadata(
            &data.document_id,
            serde_json::json!({"enrichment_status": "processing"}),
        )
        .await?;

        // Load PDF bytes from pdf_storage
        let pdf_bytes = self.load_pdf_bytes(&data.pdf_id).await?;

        // Run EdgeParser (text mode, entire PDF — text is cheap to generate)
        let converter = EdgeParsePdfConverter::default();
        let full_text = match converter
            .convert(&pdf_bytes, &PdfConversionConfig::default())
            .await
        {
            Ok(text) => text,
            Err(e) => {
                warn!(
                    document_id = %data.document_id,
                    "EdgeParser failed, marking enrichment as skipped: {}", e
                );
                self.write_metadata(
                    &data.document_id,
                    serde_json::json!({
                        "enrichment_status": "skipped",
                        "enrichment_error": format!("EdgeParser error: {}", e),
                    }),
                )
                .await?;
                return Ok(serde_json::json!({"enrichment_status": "skipped"}));
            }
        };

        if full_text.trim().is_empty() {
            self.write_metadata(
                &data.document_id,
                serde_json::json!({"enrichment_status": "skipped"}),
            )
            .await?;
            return Ok(serde_json::json!({"enrichment_status": "skipped"}));
        }

        // Truncate: MAX_TEXT_CHARS ≈ first 5 PDF pages
        let text = if full_text.len() > MAX_TEXT_CHARS {
            &full_text[..MAX_TEXT_CHARS]
        } else {
            full_text.as_str()
        };

        // Call VLM. The worker pool retries the whole task on failure;
        // we do one internal retry only for JSON-parse errors (model artifact).
        let result = match self.call_vlm(text).await {
            Ok(r) => r,
            Err(first_err) => {
                warn!(
                    document_id = %data.document_id,
                    "VLM call failed ({}), retrying once", first_err
                );
                self.call_vlm(text).await.map_err(|e| {
                    TaskError::Processing(format!("VLM enrichment failed after retry: {}", e))
                })?
            }
        };

        let keywords_capped: Vec<String> = result.keywords.into_iter().take(10).collect();

        self.write_metadata(
            &data.document_id,
            serde_json::json!({
                "enrichment_status": "completed",
                "enrichment_summary": result.summary,
                "enrichment_topic": result.topic,
                "enrichment_language": result.language,
                "enrichment_keywords": keywords_capped,
                "enrichment_completed_at": chrono::Utc::now().to_rfc3339(),
                "enrichment_error": null,
            }),
        )
        .await?;

        info!(
            document_id = %data.document_id,
            topic = %result.topic,
            language = %result.language,
            "Metadata enrichment completed"
        );

        Ok(serde_json::json!({
            "enrichment_status": "completed",
            "topic": result.topic,
            "language": result.language,
        }))
    }

    async fn on_permanent_failure(&self, task: &Task, error_msg: &str) {
        if let Ok(data) = serde_json::from_value::<MetadataEnrichData>(task.task_data.clone()) {
            let _ = self
                .write_metadata(
                    &data.document_id,
                    serde_json::json!({
                        "enrichment_status": "failed",
                        "enrichment_error": error_msg,
                    }),
                )
                .await;
        }
    }
}
```

- [ ] **Step 2: Check `PdfConversionConfig::default()` exists**

```bash
cd /home/romy/workspace/edgequake-romy/edgequake
grep -n "Default\|default()" crates/edgequake-pdf/src/backend/mod.rs | head -10
```

If `PdfConversionConfig` does not derive `Default`, add `#[derive(Default)]` to it in `crates/edgequake-pdf/src/backend/mod.rs`.

- [ ] **Step 3: Add missing imports to `Cargo.toml` of edgequake-api (if needed)**

```bash
grep "edgequake-pdf\|edgequake-storage\|chrono" crates/edgequake-api/Cargo.toml | head -10
```

`edgequake-pdf`, `edgequake-storage`, `reqwest`, and `chrono` must all be present. Add any missing ones following the existing version patterns.

- [ ] **Step 4: Compile check**

```bash
cd /home/romy/workspace/edgequake-romy/edgequake
cargo check -p edgequake-api 2>&1 | tail -30
```

Expected: compiles cleanly. Fix any type/import errors before proceeding.

- [ ] **Step 5: Commit**

```bash
cd /home/romy/workspace/edgequake-romy/edgequake
git add crates/edgequake-api/src/processor/metadata_enrich.rs \
        crates/edgequake-pdf/src/backend/mod.rs
git commit -m "feat(api): add MetadataEnrichProcessor with EdgeParser + VLM enrichment"
```

---

## Task 4: Add `enrich_queue` to `AppState`

**Files:**
- Modify: `edgequake/crates/edgequake-api/src/state/mod.rs`
- Modify: `edgequake/crates/edgequake-api/src/state/postgres.rs`
- Modify: `edgequake/crates/edgequake-api/src/state/memory.rs`

- [ ] **Step 1: Add field to `AppState` struct**

In `edgequake/crates/edgequake-api/src/state/mod.rs`, find the `pub struct AppState` block (line ~118). After the existing `pub task_queue: SharedTaskQueue,` field, add:

```rust
    /// Dedicated queue for metadata enrichment tasks (separate from main pipeline).
    pub enrich_queue: SharedTaskQueue,
```

- [ ] **Step 2: Initialize in `postgres.rs`**

In `edgequake/crates/edgequake-api/src/state/postgres.rs`, find where `task_queue` is created (line ~307):

```rust
let task_queue = Arc::new(edgequake_tasks::queue::ChannelTaskQueue::new(100));
```

Directly after it, add:

```rust
let enrich_queue = Arc::new(edgequake_tasks::queue::ChannelTaskQueue::new(200));
```

Then find the `AppState { ... }` construction (look for `task_queue,`) and add `enrich_queue,` next to it.

- [ ] **Step 3: Initialize in `memory.rs`**

Repeat the same change in `edgequake/crates/edgequake-api/src/state/memory.rs`. Search for `task_queue` and add `enrich_queue` in the same pattern.

- [ ] **Step 4: Compile check**

```bash
cd /home/romy/workspace/edgequake-romy/edgequake
cargo check -p edgequake-api 2>&1 | tail -30
```

Expected: any structs that construct `AppState` will fail with missing field errors — fix each one by adding `enrich_queue: Arc::clone(&state.enrich_queue)` or equivalent. Run `grep -rn "AppState {" crates/edgequake-api/src/` to find all construction sites.

- [ ] **Step 5: Commit**

```bash
cd /home/romy/workspace/edgequake-romy/edgequake
git add crates/edgequake-api/src/state/mod.rs \
        crates/edgequake-api/src/state/postgres.rs \
        crates/edgequake-api/src/state/memory.rs
git commit -m "feat(api): add enrich_queue to AppState for dedicated enrichment workers"
```

---

## Task 5: Enqueue enrichment task at PDF upload

**Files:**
- Modify: `edgequake/crates/edgequake-api/src/handlers/pdf_upload/helpers.rs`

The key change: pre-generate a `document_id` UUID so both the enrichment task and the `PdfProcessing` task share the same metadata record. Pass this as `existing_document_id` in `PdfProcessingData`.

- [ ] **Step 1: Write failing test**

Add the following to `edgequake/crates/edgequake-api/tests/e2e_metadata_enrichment_enqueue.rs` (create file):

```rust
//! Verifies that a PDF upload enqueues both a PdfProcessing and a MetadataEnrich task.

#[cfg(feature = "postgres")]
mod tests {
    // This is a compile-level test: verify that MetadataEnrichData can be round-tripped
    // through serde (the same path the upload handler takes when building the task).
    use edgequake_tasks::{MetadataEnrichData, TaskType};
    use uuid::Uuid;

    #[test]
    fn test_metadata_enrich_data_round_trip() {
        let doc_id = Uuid::new_v4().to_string();
        let pdf_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();

        let data = MetadataEnrichData {
            document_id: doc_id.clone(),
            pdf_id,
            tenant_id,
            workspace_id,
            max_pages: 5,
        };

        let val = serde_json::to_value(&data).unwrap();
        let back: MetadataEnrichData = serde_json::from_value(val).unwrap();

        assert_eq!(back.document_id, doc_id);
        assert_eq!(back.pdf_id, pdf_id);
        assert_eq!(back.max_pages, 5);
    }

    #[test]
    fn test_task_type_display() {
        assert_eq!(TaskType::MetadataEnrich.to_string(), "metadata_enrich");
    }
}
```

Run:

```bash
cd /home/romy/workspace/edgequake-romy/edgequake
cargo test -p edgequake-api test_metadata_enrich_data_round_trip 2>&1 | tail -20
```

Expected: PASS (these don't touch helpers.rs yet — they just validate the types).

- [ ] **Step 2: Modify `create_pdf_processing_task` in `helpers.rs`**

In `edgequake/crates/edgequake-api/src/handlers/pdf_upload/helpers.rs`:

1. Add `MetadataEnrichData` to the existing imports from `edgequake_tasks`:

```rust
use edgequake_tasks::{MetadataEnrichData, PdfProcessingData, Task, TaskStatus, TaskType};
```

2. Change the function signature to return `(track_id, document_id)` — both are needed by callers:

```rust
pub(super) async fn create_pdf_processing_task(
    state: &AppState,
    context: &TenantContext,
    pdf_id: Uuid,
    options: &PdfUploadOptions,
    workspace: Option<&Workspace>,
) -> ApiResult<(String, String)> {   // (track_id, document_id)
```

3. Inside the function, generate `document_id` before building `task_data`:

```rust
    // Pre-generate document_id so enrichment and processing tasks share the same
    // metadata record. The PdfProcessing processor uses existing_document_id to
    // avoid creating a duplicate on retry.
    let document_id = Uuid::new_v4().to_string();

    let task_data = PdfProcessingData {
        pdf_id,
        tenant_id,
        workspace_id,
        enable_vision: options.enable_vision,
        vision_provider: options.resolved_vision_provider(),
        vision_model: if options.resolved_backend(workspace)
            == edgequake_pdf::PdfParserBackend::Vision
        {
            Some(options.vision_model())
        } else {
            None
        },
        existing_document_id: Some(document_id.clone()),  // ← share the pre-generated ID
        pdf_parser_backend: options.resolved_backend(workspace),
        restart_from_scratch: false,
    };
```

4. After the existing `state.task_queue.send(task).await?;`, enqueue the enrichment task:

```rust
    // Enqueue metadata enrichment task to the dedicated enrichment pool.
    // This runs before (and independently of) the full PdfProcessing pipeline.
    let enrich_data = MetadataEnrichData {
        document_id: document_id.clone(),
        pdf_id,
        tenant_id,
        workspace_id,
        max_pages: 5,
    };
    let enrich_task = Task {
        track_id: format!("metadata_enrich-{}", Uuid::new_v4()),
        tenant_id,
        workspace_id,
        task_type: TaskType::MetadataEnrich,
        status: TaskStatus::Pending,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        started_at: None,
        completed_at: None,
        error_message: None,
        error: None,
        retry_count: 0,
        max_retries: 3,
        consecutive_timeout_failures: 0,
        circuit_breaker_tripped: false,
        task_data: serde_json::to_value(&enrich_data)
            .map_err(|e| ApiError::Internal(format!("Failed to serialize enrich data: {}", e)))?,
        metadata: None,
        progress: None,
        result: None,
    };

    // Store enrichment task in DB so it survives restarts
    state
        .task_storage
        .create_task(&enrich_task)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to create enrichment task: {}", e)))?;

    state
        .enrich_queue
        .send(enrich_task)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to queue enrichment task: {}", e)))?;

    // Write initial enrichment_status to metadata so clients can poll it
    let metadata_key = format!("{}-metadata", document_id);
    let _ = state
        .kv_storage
        .upsert(&[(metadata_key, serde_json::json!({"enrichment_status": "pending"}))])
        .await;
```

5. Change the return value at the end of the function:

```rust
    Ok((track_id, document_id))
```

- [ ] **Step 3: Fix callers of `create_pdf_processing_task`**

Find every call site:

```bash
grep -rn "create_pdf_processing_task" /home/romy/workspace/edgequake-romy/edgequake/crates/
```

Each call currently expects a `String` (track_id). Update them to destructure the tuple:

```rust
let (track_id, _document_id) = create_pdf_processing_task(state, &context, pdf_id, &options, workspace.as_ref()).await?;
```

(Or use `document_id` in the response if the handler wants to return it.)

- [ ] **Step 4: Compile check**

```bash
cd /home/romy/workspace/edgequake-romy/edgequake
cargo check -p edgequake-api 2>&1 | tail -30
```

Expected: clean. Fix any remaining type errors.

- [ ] **Step 5: Commit**

```bash
cd /home/romy/workspace/edgequake-romy/edgequake
git add crates/edgequake-api/src/handlers/pdf_upload/helpers.rs \
        crates/edgequake-api/src/handlers/pdf_upload/upload.rs \
        tests/e2e_metadata_enrichment_enqueue.rs
git commit -m "feat(api): enqueue MetadataEnrich task alongside PdfProcessing on PDF upload"
```

---

## Task 6: Start enrichment worker pool in `main.rs`

**Files:**
- Modify: `edgequake/src/main.rs`

- [ ] **Step 1: Read enrichment config and start pool after the main worker pool**

In `edgequake/src/main.rs`, find the section where the main `worker_pool.start()` is called (around line 640). Directly after it, add:

```rust
    // ── Enrichment worker pool ────────────────────────────────────────────────
    // Separate pool with 4 workers (default) dedicated exclusively to
    // MetadataEnrich tasks. Workers are IO-bound (one VLM call per task)
    // so 4 is sufficient for most deployments. Override via ENRICHMENT_CONCURRENT.
    let enrichment_config = edgequake_api::processor::enrichment_config::EnrichmentConfig::from_env();
    let enrich_concurrent = enrichment_config.concurrent;

    let enrich_processor = Arc::new(
        edgequake_api::processor::metadata_enrich::MetadataEnrichProcessor::new(
            Arc::clone(&state.kv_storage) as Arc<dyn edgequake_storage::traits::KVStorage>,
            state.pdf_storage.as_ref().map(Arc::clone),
            enrichment_config,
        ),
    );

    let enrich_pool_config = WorkerPoolConfig {
        num_workers: enrich_concurrent,
        auto_retry: true,
        initial_retry_delay_ms: 1_000,
        max_retry_delay_ms: 10_000,
        backoff_multiplier: 2.0,
        max_tasks_per_tenant: enrich_concurrent, // no fairness limit needed for enrichment
        processing_timeout_secs: 300, // 5 min is plenty for 5-page enrichment
    };

    let mut enrich_pool = WorkerPool::new(
        enrich_pool_config,
        Arc::clone(&state.enrich_queue) as Arc<dyn edgequake_tasks::TaskQueue>,
        Arc::clone(&state.task_storage) as Arc<dyn edgequake_tasks::TaskStorage>,
        enrich_processor,
    );

    info!(
        "Starting enrichment worker pool with {} workers",
        enrich_concurrent
    );
    enrich_pool.start();
```

- [ ] **Step 2: Add `enrich_pool` to the shutdown sequence**

Find the server shutdown logic in `main.rs` (look for `worker_pool.shutdown()` or server termination). Add:

```rust
    enrich_pool.shutdown().await;
```

alongside the existing `worker_pool.shutdown().await;`.

- [ ] **Step 3: Requeue pending enrichment tasks on startup**

The existing `requeue_pending_tasks` function loads ALL pending tasks into `state.task_queue`. We need enrichment tasks to go into `state.enrich_queue` instead. Find the call to `requeue_pending_tasks` and modify it, or add a second call:

```rust
    // Requeue pending enrichment tasks into the enrichment queue specifically
    if let Err(e) = requeue_pending_tasks(
        Arc::clone(&state.task_storage) as Arc<dyn TaskStorage>,
        Arc::clone(&state.enrich_queue) as Arc<dyn TaskQueue>,
        // Pass a task type filter if the function supports it, otherwise add filtering
    ).await {
        warn!("Failed to requeue pending enrichment tasks (non-fatal): {}", e);
    }
```

Check the signature of `requeue_pending_tasks`. If it doesn't filter by task type, add a `TaskType` filter parameter or create a `requeue_pending_enrichment_tasks` variant that filters for `TaskType::MetadataEnrich`. Look at the existing `TaskFilter` struct:

```bash
grep -n "TaskFilter\|task_type" /home/romy/workspace/edgequake-romy/edgequake/crates/edgequake-tasks/src/storage.rs | head -20
```

- [ ] **Step 4: Full compile check**

```bash
cd /home/romy/workspace/edgequake-romy/edgequake
cargo build 2>&1 | tail -30
```

Expected: clean build.

- [ ] **Step 5: Commit**

```bash
cd /home/romy/workspace/edgequake-romy/edgequake
git add src/main.rs
git commit -m "feat: start dedicated enrichment worker pool on server startup"
```

---

## Task 7: End-to-end smoke test

**Files:**
- Create: `edgequake/crates/edgequake-api/tests/e2e_enrichment_processor.rs`

This test exercises `MetadataEnrichProcessor` with a mock VLM server and an in-memory storage mock. It verifies that metadata fields are written correctly.

- [ ] **Step 1: Write the test**

Create `edgequake/crates/edgequake-api/tests/e2e_enrichment_processor.rs`:

```rust
//! Tests that MetadataEnrichProcessor writes the correct metadata fields.
//! Uses a real in-memory KV storage and mocks the VLM HTTP endpoint with wiremock.

// NOTE: This test requires the `wiremock` dev-dependency.
// Check Cargo.toml: if absent, add `wiremock = "0.6"` under [dev-dependencies].

#[cfg(test)]
mod tests {
    use edgequake_tasks::{MetadataEnrichData, TaskProcessor, TaskType};
    use edgequake_tasks::types::Task;
    use uuid::Uuid;

    /// Verify MetadataEnrichData deserialization from JSON (worker-path simulation).
    #[test]
    fn test_enrich_data_from_task_data() {
        let document_id = Uuid::new_v4().to_string();
        let pdf_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();

        let data = MetadataEnrichData {
            document_id: document_id.clone(),
            pdf_id,
            tenant_id,
            workspace_id,
            max_pages: 5,
        };

        let task_data = serde_json::to_value(&data).unwrap();
        let recovered: MetadataEnrichData = serde_json::from_value(task_data).unwrap();

        assert_eq!(recovered.document_id, document_id);
        assert_eq!(recovered.max_pages, 5);
    }

    /// Verify that `on_permanent_failure` writes `enrichment_status: "failed"`.
    #[tokio::test]
    async fn test_on_permanent_failure_writes_failed_status() {
        use edgequake_api::processor::enrichment_config::EnrichmentConfig;
        use edgequake_api::processor::metadata_enrich::MetadataEnrichProcessor;
        use edgequake_storage::MemoryKVStorage; // exported from crate root
        use std::sync::Arc;

        let kv = Arc::new(MemoryKVStorage::new());
        let config = EnrichmentConfig {
            vlm_base_url: "http://localhost:1".to_string(), // unreachable — not called
            vlm_model: "test".to_string(),
            max_pages: 5,
            concurrent: 1,
        };
        let processor = MetadataEnrichProcessor::new(Arc::clone(&kv) as Arc<_>, None, config);

        let document_id = Uuid::new_v4().to_string();
        let task_data = serde_json::to_value(MetadataEnrichData {
            document_id: document_id.clone(),
            pdf_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            workspace_id: Uuid::new_v4(),
            max_pages: 5,
        })
        .unwrap();

        let task = Task {
            track_id: format!("metadata_enrich-{}", Uuid::new_v4()),
            tenant_id: Uuid::new_v4(),
            workspace_id: Uuid::new_v4(),
            task_type: TaskType::MetadataEnrich,
            status: edgequake_tasks::TaskStatus::Failed,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            error_message: None,
            error: None,
            retry_count: 3,
            max_retries: 3,
            consecutive_timeout_failures: 0,
            circuit_breaker_tripped: false,
            task_data,
            metadata: None,
            progress: None,
            result: None,
        };

        processor.on_permanent_failure(&task, "retries exhausted").await;

        // Check that KV was updated with failed status
        let key = format!("{}-metadata", document_id);
        let stored = kv.get_by_id(&key).await.unwrap().unwrap();
        assert_eq!(stored["enrichment_status"], "failed");
        assert!(stored["enrichment_error"].as_str().unwrap().contains("retries exhausted"));
    }
}
```

- [ ] **Step 2: Check `MemoryKVStorage` export path**

```bash
grep -rn "pub struct MemoryKVStorage\|pub use.*MemoryKVStorage" \
  /home/romy/workspace/edgequake-romy/edgequake/crates/edgequake-storage/src/ | head -5
```

Adjust the import path in the test if it differs.

- [ ] **Step 3: Run the tests**

```bash
cd /home/romy/workspace/edgequake-romy/edgequake
cargo test -p edgequake-api test_on_permanent_failure_writes_failed_status 2>&1 | tail -20
cargo test -p edgequake-api test_enrich_data_from_task_data 2>&1 | tail -20
```

Expected: both PASS.

- [ ] **Step 4: Run full test suite to catch regressions**

```bash
cd /home/romy/workspace/edgequake-romy/edgequake
cargo test -p edgequake-tasks -p edgequake-api 2>&1 | tail -40
```

Expected: all previously-passing tests still pass.

- [ ] **Step 5: Commit**

```bash
cd /home/romy/workspace/edgequake-romy/edgequake
git add crates/edgequake-api/tests/e2e_enrichment_processor.rs \
        tests/e2e_metadata_enrichment_enqueue.rs
git commit -m "test: add MetadataEnrichProcessor unit and enqueue tests"
```

---

## .env additions

Add to `/home/romy/workspace/edgequake-romy/.env.example` (and your local `.env`):

```env
# PDF Metadata Enrichment Pipeline
# Workers dedicated to pre-pipeline summary extraction
ENRICHMENT_CONCURRENT=4
# OpenAI-compatible local VLM endpoint
ENRICHMENT_VLM_BASE_URL=http://localhost:11434/v1
# Model name (must accept text input; vision capability not required)
ENRICHMENT_VLM_MODEL=llava:7b
# Number of PDF pages to extract text from for enrichment
ENRICHMENT_MAX_PAGES=5
```

---

## Verification Checklist (after all tasks)

Run these to confirm the full feature works:

```bash
# 1. Compile cleanly
cd /home/romy/workspace/edgequake-romy/edgequake
cargo build 2>&1 | tail -5

# 2. All unit tests pass
cargo test -p edgequake-tasks -p edgequake-api 2>&1 | grep -E "FAILED|passed|failed" | tail -10

# 3. Clippy clean
cargo clippy -p edgequake-tasks -p edgequake-api --all-targets -- -D warnings 2>&1 | tail -20
```

Manual smoke test (requires running server + local Ollama):

```bash
# Upload a PDF
curl -X POST http://localhost:8080/api/v1/documents/pdf \
  -H "X-Workspace-ID: <workspace-id>" \
  -F "file=@/path/to/sample.pdf"

# Poll until enrichment_status = "completed" (should appear within ~10s)
curl http://localhost:8080/api/v1/documents/<document-id> | jq '{
  enrichment_status: .enrichment_status,
  topic: .enrichment_topic,
  summary: .enrichment_summary,
  language: .enrichment_language,
  keywords: .enrichment_keywords
}'
```

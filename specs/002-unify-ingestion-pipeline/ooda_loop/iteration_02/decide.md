# Iteration 02 - Decide

**Mission File**: `./specs/002-unify-ingestion-pipeline.md`

**Date**: 2026-02-01

---

## Decision Matrix

| Change | Impact | Effort | Priority |
|--------|--------|--------|----------|
| Add `source_type`, `current_stage` to `DocumentSummary` | High | Low | 1 |
| Update `get_track_status` to return new fields | High | Low | 2 |
| Store `source_type` on document upload | High | Low | 3 |
| Store `source_type` on PDF upload | High | Low | 4 |
| Update frontend `Document` type | High | Low | 5 |
| Update `DocumentManager` to show unified status | Medium | Medium | 6 |

---

## Action Plan for Iteration 02

### Priority 1: Update DocumentSummary Type

**File**: `edgequake/crates/edgequake-api/src/handlers/documents_types.rs`

Add new fields to `DocumentSummary`:

```rust
/// Document summary for list views.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DocumentSummary {
    // ... existing fields ...
    
    /// Document source type (pdf, markdown, text)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "pdf")]
    pub source_type: Option<String>,
    
    /// Current ingestion stage (unified with backend UnifiedStage)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "extracting")]
    pub current_stage: Option<String>,
    
    /// Progress within current stage (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = json!(0.45))]
    pub stage_progress: Option<f32>,
    
    /// Human-readable stage message
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "Extracting entities from chunk 5/12")]
    pub stage_message: Option<String>,
}
```

### Priority 2: Update get_track_status to Read New Fields

**File**: `edgequake/crates/edgequake-api/src/handlers/documents.rs`

In `get_track_status`, add field extraction:

```rust
track_docs.push(DocumentSummary {
    // ... existing fields ...
    source_type: obj.get("source_type").and_then(|v| v.as_str()).map(String::from),
    current_stage: obj.get("current_stage").and_then(|v| v.as_str()).map(String::from),
    stage_progress: obj.get("stage_progress").and_then(|v| v.as_f64()).map(|n| n as f32),
    stage_message: obj.get("stage_message").and_then(|v| v.as_str()).map(String::from),
});
```

### Priority 3: Store source_type on Document Upload

**File**: `edgequake/crates/edgequake-api/src/handlers/documents.rs`

In `upload_document`, add `source_type`:

```rust
let doc_metadata = serde_json::json!({
    "id": document_id,
    "title": request.title,
    "source_type": "markdown",  // or "text" based on content type
    "current_stage": "uploading",
    "status": initial_status,
    // ... existing fields
});
```

### Priority 4: Store source_type on PDF Upload

**File**: `edgequake/crates/edgequake-api/src/handlers/pdf_upload.rs`

When creating PDF metadata, include source_type:

```rust
// In task creation or callback initialization
metadata.insert("source_type", "pdf");
metadata.insert("current_stage", "uploading");
```

### Priority 5: Update Frontend Document Type

**File**: `edgequake_webui/src/types/index.ts`

Add new fields to Document interface:

```typescript
export interface Document {
  // ... existing fields ...
  
  /** Document source type (pdf, markdown, text) */
  source_type?: SourceType;
  
  /** Current ingestion stage */
  current_stage?: IngestionStage;
  
  /** Progress within current stage (0.0-1.0) */
  stage_progress?: number;
  
  /** Human-readable stage message */
  stage_message?: string;
}
```

### Priority 6: Update DocumentManager Display

**File**: `edgequake_webui/src/components/documents/document-manager.tsx`

Use `current_stage` instead of `status` for badge:

```tsx
<StatusBadge 
  status={normalizeStatus(doc.current_stage || doc.status)}
  tooltip={doc.stage_message}
/>
```

---

## Commit Plan

```
OODA-02: Add source_type and current_stage to document tracking

Backend:
- Added source_type, current_stage, stage_progress, stage_message to DocumentSummary
- Updated get_track_status to extract new fields from metadata
- Store source_type="markdown" on text/markdown upload
- Store source_type="pdf" on PDF upload

Frontend:
- Added new fields to Document interface
- StatusBadge now uses current_stage when available

@implements SPEC-002: Unified Ingestion Pipeline
```

---

## Verification Checklist

- [ ] `cargo build` passes
- [ ] `cargo test` passes (document handler tests)
- [ ] Frontend builds (`pnpm build`)
- [ ] Manual test: Upload markdown → check track status shows source_type
- [ ] Manual test: Upload PDF → check track status shows source_type

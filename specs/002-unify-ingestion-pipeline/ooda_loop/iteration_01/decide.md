# Iteration 01 - Decide

**Mission File**: `./specs/002-unify-ingestion-pipeline.md`

**Date**: 2026-02-01

---

## Decision Matrix

| Change                                    | Impact | Effort | Priority |
| ----------------------------------------- | ------ | ------ | -------- |
| Create `IngestionStage` enum              | High   | Low    | 1        |
| Create `IngestionProgress` struct         | High   | Low    | 2        |
| Create `IngestionError` struct            | Medium | Low    | 3        |
| Update frontend `statusConfig`            | High   | Low    | 4        |
| Update PDF handler progress emission      | Medium | Medium | 5        |
| Update document handler progress emission | Medium | Medium | 6        |
| Add E2E tests                             | High   | Medium | 7        |

---

## Action Plan for Iteration 01

### Priority 1: Create Unified Types (Backend)

**File**: `edgequake/crates/edgequake-pipeline/src/ingestion_types.rs` (NEW)

```rust
//! Unified ingestion types for PDF and Markdown processing.
//!
//! @implements SPEC-002: Unified Ingestion Pipeline

/// Source type for ingestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Pdf,
    Markdown,
    Text,
}

/// Unified ingestion stage.
/// Used by both PDF and Markdown flows for consistent progress tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum IngestionStage {
    Uploading,      // File/content received
    Converting,     // PDF → Markdown (PDF only)
    Preprocessing,  // Validation, parsing
    Chunking,       // Document splitting
    Extracting,     // Entity/relationship extraction
    Gleaning,       // Re-extraction for missed entities
    Merging,        // Graph merge
    Summarizing,    // Description summarization
    Embedding,      // Vector generation
    Storing,        // Persist to storage
    Completed,      // Successfully finished
    Failed,         // Error state
}

/// Progress for a single stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageProgress {
    pub stage: IngestionStage,
    pub status: StageStatus,
    pub progress: f32,           // 0.0 to 1.0
    pub message: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub duration_ms: Option<u64>,
}

/// Status of a stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StageStatus {
    Pending,
    Running,
    Completed,
    Skipped,
    Failed,
}

/// Unified ingestion progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionProgress {
    pub track_id: String,
    pub document_id: Option<String>,
    pub source_type: SourceType,
    pub filename: Option<String>,
    pub current_stage: IngestionStage,
    pub stages: Vec<StageProgress>,
    pub overall_progress: f32,
    pub message: String,
    pub error: Option<IngestionError>,
    pub cost_usd: Option<f64>,
}

/// Unified ingestion error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionError {
    pub code: String,
    pub message: String,
    pub stage: IngestionStage,
    pub details: Option<serde_json::Value>,
    pub recoverable: bool,
}
```

### Priority 2: Update Pipeline Module Exports

**File**: `edgequake/crates/edgequake-pipeline/src/lib.rs`

Add new module and exports:

```rust
pub mod ingestion_types;

pub use ingestion_types::{
    IngestionStage,
    IngestionProgress,
    IngestionError,
    SourceType,
    StageProgress,
    StageStatus,
};
```

### Priority 3: Update Frontend Types

**File**: `edgequake_webui/src/types/ingestion.ts` (UPDATE)

Align with backend `IngestionStage`:

```typescript
export type IngestionStage =
  | "uploading"
  | "converting"
  | "preprocessing"
  | "chunking"
  | "extracting"
  | "gleaning"
  | "merging"
  | "summarizing"
  | "embedding"
  | "storing"
  | "completed"
  | "failed";

export type SourceType = "pdf" | "markdown" | "text";
```

### Priority 4: Update StatusBadge Component

**File**: `edgequake_webui/src/components/documents/status-badge.tsx`

Update `statusConfig` to match all `IngestionStage` values:

```typescript
const statusConfig = {
  // Upload stage
  uploading: {
    icon: Upload,
    color: "bg-blue-400",
    textColor: "...",
    label: "Uploading",
    animate: true,
  },

  // Conversion stage (PDF only)
  converting: {
    icon: FileType,
    color: "bg-indigo-500",
    textColor: "...",
    label: "Converting PDF",
    animate: true,
  },

  // Processing stages
  preprocessing: {
    icon: Loader2,
    color: "bg-blue-500",
    textColor: "...",
    label: "Preprocessing",
    animate: true,
  },
  chunking: {
    icon: Scissors,
    color: "bg-blue-400",
    textColor: "...",
    label: "Chunking",
    animate: true,
  },
  extracting: {
    icon: Brain,
    color: "bg-purple-500",
    textColor: "...",
    label: "Extracting",
    animate: true,
  },
  gleaning: {
    icon: Search,
    color: "bg-purple-400",
    textColor: "...",
    label: "Gleaning",
    animate: true,
  },
  merging: {
    icon: GitMerge,
    color: "bg-amber-500",
    textColor: "...",
    label: "Merging",
    animate: true,
  },
  summarizing: {
    icon: FileText,
    color: "bg-orange-500",
    textColor: "...",
    label: "Summarizing",
    animate: true,
  },
  embedding: {
    icon: Cpu,
    color: "bg-cyan-500",
    textColor: "...",
    label: "Embedding",
    animate: true,
  },
  storing: {
    icon: Database,
    color: "bg-teal-500",
    textColor: "...",
    label: "Storing",
    animate: true,
  },

  // Terminal states
  completed: {
    icon: CheckCircle,
    color: "bg-green-500",
    textColor: "...",
    label: "Completed",
    animate: false,
  },
  failed: {
    icon: XCircle,
    color: "bg-red-500",
    textColor: "...",
    label: "Failed",
    animate: false,
  },

  // Legacy (backward compat)
  pending: {
    icon: Clock,
    color: "bg-yellow-500",
    textColor: "...",
    label: "Pending",
    animate: false,
  },
  processing: {
    icon: Loader2,
    color: "bg-blue-500",
    textColor: "...",
    label: "Processing",
    animate: true,
  },
  indexing: {
    icon: Database,
    color: "bg-teal-500",
    textColor: "...",
    label: "Indexing",
    animate: true,
  },
  indexed: {
    icon: CheckCircle,
    color: "bg-green-500",
    textColor: "...",
    label: "Indexed",
    animate: false,
  },
  cancelled: {
    icon: StopCircle,
    color: "bg-orange-500",
    textColor: "...",
    label: "Cancelled",
    animate: false,
  },
};
```

---

## Commit Plan

```
OODA-01: Add unified ingestion types (IngestionStage, IngestionProgress, IngestionError)

- Add edgequake-pipeline/src/ingestion_types.rs
- Export types from lib.rs
- Update frontend types/ingestion.ts
- Update status-badge.tsx statusConfig
```

---

## Verification Checklist

- [ ] `cargo build` passes
- [ ] `cargo test` passes
- [ ] Frontend builds (`pnpm build`)
- [ ] StatusBadge renders all stages correctly
- [ ] No breaking changes to existing endpoints

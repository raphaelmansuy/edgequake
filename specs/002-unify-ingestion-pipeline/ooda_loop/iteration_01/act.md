# Iteration 01 - Act

**Mission File**: `./specs/002-unify-ingestion-pipeline.md`

**Date**: 2026-02-01

---

## Changes Implemented

### 1. Created Unified Ingestion Types (Backend)

**File**: [edgequake/crates/edgequake-pipeline/src/ingestion_types.rs](edgequake/crates/edgequake-pipeline/src/ingestion_types.rs) (NEW - 519 lines)

```rust
// Key types added:
pub enum SourceType { Pdf, Markdown, Text }
pub enum UnifiedStage { Uploading, Converting, Preprocessing, ... }
pub struct IngestionProgress { ... }
pub struct IngestionError { ... }
pub mod error_codes { ... }
```

**WHY**: Provides unified stage definitions used by both PDF and Markdown flows.

### 2. Updated Pipeline Module Exports

**File**: [edgequake/crates/edgequake-pipeline/src/lib.rs](edgequake/crates/edgequake-pipeline/src/lib.rs#L60-L66)

Added exports:
```rust
pub use ingestion_types::{
    error_codes, SourceType, UnifiedStage,
    IngestionError as UnifiedIngestionError,
    IngestionProgress as UnifiedIngestionProgress,
    StageProgress as UnifiedStageProgress,
    StageStatus as UnifiedStageStatus,
};
```

### 3. Updated Frontend Types

**File**: [edgequake_webui/src/types/ingestion.ts](edgequake_webui/src/types/ingestion.ts#L21-L68)

```typescript
export type SourceType = "pdf" | "markdown" | "text";

export type IngestionStage =
  | "uploading"      // New
  | "converting"     // New (PDF only)
  | "preprocessing"
  | "chunking"
  | "extracting"
  | "gleaning"
  | "merging"
  | "summarizing"
  | "embedding"
  | "storing"        // New (was indexing)
  | "completed"
  | "failed"
  // Legacy aliases
  | "pending"
  | "indexing";
```

### 4. Updated StatusBadge Component

**File**: [edgequake_webui/src/components/documents/status-badge.tsx](edgequake_webui/src/components/documents/status-badge.tsx#L37-L70)

Added icons and colors for all unified stages:
- `uploading` → Upload icon, blue
- `converting` → FileText icon, indigo
- `preprocessing` → Loader2 icon, blue
- `gleaning` → Search icon, purple
- `merging` → GitMerge icon, amber
- `summarizing` → FileText icon, orange
- `storing` → Database icon, teal

### 5. Updated StageIndicator Component

**File**: [edgequake_webui/src/components/progress/stage-indicator.tsx](edgequake_webui/src/components/progress/stage-indicator.tsx#L42-L60)

Updated `STAGE_LABELS` to include all unified stages.

---

## Verification

### Backend Build ✅

```bash
cd edgequake && cargo build
# Finished `dev` profile in 1m 36s
```

### Frontend Type Check ✅

```bash
cd edgequake_webui && pnpm tsc --noEmit
# No errors
```

---

## Files Changed

| File | Action | Lines |
|------|--------|-------|
| `edgequake-pipeline/src/ingestion_types.rs` | Created | 519 |
| `edgequake-pipeline/src/lib.rs` | Modified | +8 |
| `edgequake_webui/src/types/ingestion.ts` | Modified | +20 |
| `edgequake_webui/src/components/documents/status-badge.tsx` | Modified | +35 |
| `edgequake_webui/src/components/progress/stage-indicator.tsx` | Modified | +9 |

---

## Next Steps (Iteration 02)

1. Update PDF handler to emit `UnifiedStage` events
2. Update document handler to emit `UnifiedStage` events  
3. Update API progress endpoints to return unified format
4. Add E2E tests for PDF and Markdown upload flows

---

## Commit

```
OODA-01: Add unified ingestion types for PDF and Markdown

- Added edgequake-pipeline/src/ingestion_types.rs with:
  * SourceType enum (Pdf, Markdown, Text)
  * UnifiedStage enum (12 stages including converting for PDF)
  * IngestionProgress struct for progress tracking
  * IngestionError struct for unified error handling
  * Conversion methods between UnifiedStage and PipelineStage

- Updated frontend types:
  * Added SourceType to types/ingestion.ts
  * Extended IngestionStage with uploading, converting, storing
  * Maintained backward compatibility with pending/indexing aliases

- Updated UI components:
  * StatusBadge now shows all unified stages with distinct icons
  * StageIndicator labels updated for all stages

@implements SPEC-002: Unified Ingestion Pipeline
@implements FEAT0001: Document Ingestion Pipeline
```

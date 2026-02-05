# OODA-44: Orient - Deprecation Approach

## Date: 2026-02-05

## Decision: Deprecate at Module Level

Instead of marking every individual function deprecated (too verbose), we'll:

1. Add deprecation warning to the main `ExtractionEngine` struct
2. Keep other modules internal (not re-exported publicly)
3. Document the deprecation path

---

## Rationale

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Deprecation Strategy Comparison                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Option A: Deprecate every function                                         │
│    - 100+ deprecation attributes                                            │
│    - Noisy build output                                                     │
│    - High maintenance burden                                                │
│    ❌ REJECTED                                                              │
│                                                                             │
│  Option B: Deprecate only public API (ExtractionEngine)                     │
│    - 1 deprecation attribute                                                │
│    - Clear migration message                                                │
│    - Internal modules can be removed later without API churn                │
│    ✅ SELECTED                                                              │
│                                                                             │
│  Option C: Feature flag removal                                             │
│    - Breaking change                                                        │
│    - Would require major version bump                                       │
│    ❌ Too aggressive for now                                                │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Implementation Plan

### Step 1: Add deprecation to ExtractionEngine

File: `backend/extraction_engine.rs`

````rust
/// Legacy PDF extraction backend using lopdf library.
///
/// # Deprecated
///
/// This backend is deprecated in favor of [`PdfiumBackend`] which provides
/// more accurate font style detection using PDFium's font descriptor flags.
///
/// ## Migration
///
/// ```rust
/// // Before (deprecated)
/// let backend = ExtractionEngine::new();
///
/// // After (recommended)
/// let backend = PdfiumBackend::new()?;
/// ```
///
/// ## Why PDFium is preferred
///
/// - Font descriptor flags: Accurate bold/italic detection
/// - Better character positions: More precise bounding boxes
/// - Active maintenance: Chromium's PDF engine
#[deprecated(
    since = "0.2.0",
    note = "Use PdfiumBackend instead for more accurate font style detection. \
            See docs for migration guide."
)]
pub struct ExtractionEngine { ... }
````

### Step 2: Add doc comment to backend/mod.rs

Add a module-level warning explaining the deprecation.

### Step 3: Update CHANGELOG

Document the deprecation in the mission spec.

---

## Expected Outcome

When users compile with `--features lopdf`:

- Warning: `ExtractionEngine is deprecated`
- Note: "Use PdfiumBackend instead"

When users compile with default features:

- PdfiumBackend is used automatically
- No deprecation warnings (preferred path)

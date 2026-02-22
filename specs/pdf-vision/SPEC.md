# SPEC-040: Migrate to `edgequake-pdf2md` — VLM-Native PDF Conversion

**Status:** Implementation Complete — `edgequake-pdf2md` v0.2 integrated, all core tasks done  
**Created:** 2026-02-19  
**Branch:** `feat/pdf-llm-vision`  
**Replaces:** Internal `edgequake/crates/edgequake-pdf` (moved to `legacy/`)

### Upstream blockers (filed as `edgequake-pdf2md` issues)

| Issue                                                            | Title                                                              | Blocker?                                          |
| ---------------------------------------------------------------- | ------------------------------------------------------------------ | ------------------------------------------------- |
| [#1](https://github.com/raphaelmansuy/edgequake-pdf2md/issues/1) | `ConversionProgressCallback` trait for per-page progress           | **Hard** — UX regresses to spinner otherwise      |
| [#2](https://github.com/raphaelmansuy/edgequake-pdf2md/issues/2) | `convert_from_bytes()` API — no tempfile boilerplate               | **Hard** — callers must manage tempfiles manually |
| [#3](https://github.com/raphaelmansuy/edgequake-pdf2md/issues/3) | Document `Arc<dyn LLMProvider>` injection via builder              | Soft — workaround via `provider_name`             |
| [#4](https://github.com/raphaelmansuy/edgequake-pdf2md/issues/4) | `Pdf2MdError` ergonomics (partial failure, rate-limit distinction) | Soft — v0.2.x improvement                         |

**Implementation gate:** Begin code changes once issues #1 and #2 are resolved in a released version of `edgequake-pdf2md`.

---

## 1. Problem Statement

The legacy `edgequake-pdf` crate extracts text from PDFs using a pdfium +
text-layer approach and optionally routes through a vision LLM only as a
feature-gated fallback (`vision` Cargo feature). This has three key deficiencies:

| Deficiency                                                                                    | Impact                                          |
| --------------------------------------------------------------------------------------------- | ----------------------------------------------- |
| Text-layer extraction produces garbled output on complex layouts (multi-column, tables, math) | Poor RAG quality                                |
| Vision path hard-codes `gpt-4o-mini` in code rather than respecting workspace config          | Operator cannot control cost/quality            |
| Progress callback trait is tightly coupled to the internal extraction crate                   | Any crate change breaks the API progress system |

The new `edgequake-pdf2md` crate (`crates.io/crates/edgequake-pdf2md`, v0.1.0)
fixes all three: it uses VLM vision as the _primary and only_ path, exposes a
clean `ConversionConfig` builder, and accepts a pre-built `Arc<dyn LLMProvider>`
so the workspace-level provider is injected from outside.

---

## 2. Goals

- **G1** — Replace `edgequake-pdf` with `edgequake-pdf2md` in the workspace build.
- **G2** — LLM vision provider/model configurable at workspace level (UI + API + env vars).
- **G3** — Default model: `gpt-4.1-nano` (cheap, fast, vision-capable).
- **G4** — Legacy crate preserved in `./legacy/edgequake-pdf` (no deletion).
- **G5** — All existing API endpoints, SDKs, and WebUI remain backward-compatible.
- **G6** — Makefile, `.env.example`, and WebUI docs updated for vision configuration.

---

## 3. Non-Goals

- Changing the PDF upload HTTP endpoint (`POST /v1/workspaces/{id}/documents/pdf`).
- Removing text-extraction altogether (the new crate is always vision-based).
- Supporting streaming per-page progress callbacks (new crate does not expose this).

---

## 4. Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│  workspace config                                                        │
│  vision_provider: "openai"   (or "anthropic", "ollama", "gemini", …)   │
│  vision_model:    "gpt-4.1-nano"                                        │
└───────────────────────────┬─────────────────────────────────────────────┘
                            │  env-var fallback:
                            │  EDGEQUAKE_VISION_PROVIDER / EDGEQUAKE_VISION_MODEL
                            ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  edgequake-api  processor.rs                                            │
│                                                                         │
│  1. write pdf_data bytes → tempfile                                     │
│  2. build ConversionConfig:                                             │
│       .provider(arc_llm_provider)  ← workspace vision LLM              │
│       .model(vision_model)                                              │
│       .dpi(150)                                                         │
│  3. edgequake_pdf2md::convert(tempfile_path, &config).await?           │
│  4. emit basic progress events (start / complete / error)               │
│  5. return output.markdown                                               │
└───────────────────────────┬─────────────────────────────────────────────┘
                            │
                            ▼
          edgequake-pdf2md  (crates.io, no-default-features)
          PDF → pdfium → PNG pages → base64 → VLM API → post-process → Markdown
```

---

## 5. New Library API Surface (edgequake-pdf2md v0.1.0)

### 5.1 Key types

```rust
// Entry point
pub async fn convert(
    input_str: impl AsRef<str>,   // local file path OR https:// URL
    config: &ConversionConfig,
) -> Result<ConversionOutput, Pdf2MdError>;

// Output
pub struct ConversionOutput {
    pub markdown: String,
    pub pages: Vec<PageResult>,
    pub stats: ConversionStats,
    pub metadata: DocumentMetadata,
}

// Config (builder pattern)
let config = ConversionConfig::builder()
    .provider(arc_llm_provider)   // inject pre-built provider
    .model("gpt-4.1-nano")
    .dpi(150)
    .build()?;
```

### 5.2 Provider injection

`ConversionConfig.provider: Option<Arc<dyn LLMProvider>>` — takes precedence
over `provider_name` and env-var auto-detection. This is the key integration
point: we inject the workspace's vision LLM provider from processor.rs.

### 5.3 No ProgressCallback

The new crate does not expose a per-page progress callback. We must:

- Emit a **"converting"** stage event at the start of PDF processing.
- Emit a **"completed"** or **"failed"** event when `convert()` returns.
- Remove the `ProgressCallback` trait import from `pipeline_progress_callback.rs`.

---

## 6. New Environment Variables

| Variable                    | Default              | Description                     |
| --------------------------- | -------------------- | ------------------------------- |
| `EDGEQUAKE_VISION_PROVIDER` | `openai`             | VLM provider for PDF conversion |
| `EDGEQUAKE_VISION_MODEL`    | `gpt-4.1-nano`       | VLM model for PDF conversion    |
| `PDFIUM_DYNAMIC_LIB_PATH`   | auto-set by Makefile | Path to libpdfium.dylib / .so   |

> **Note:** `OPENAI_API_KEY` (or `ANTHROPIC_API_KEY` / `GEMINI_API_KEY`) is still
> required for the corresponding provider.

---

## 7. Workspace Schema Extension

Add two optional fields to the `Workspace` struct and all related request/response
types. Both fall back to the env vars above when absent.

```rust
// edgequake-core/src/types/multitenancy.rs  →  struct Workspace
pub vision_provider: Option<String>,   // "openai" | "anthropic" | "ollama" | "gemini"
pub vision_model:    Option<String>,   // "gpt-4.1-nano" | "claude-haiku-4-20250514" | …
```

API DTOs updated: `CreateWorkspaceRequest`, `UpdateWorkspaceRequest`, `WorkspaceResponse`.

Default resolution order (processor.rs):

```
workspace.vision_model
  →  EDGEQUAKE_VISION_MODEL
  →  "gpt-4.1-nano"

workspace.vision_provider
  →  EDGEQUAKE_VISION_PROVIDER
  →  "openai"
```

---

## 8. Changes by File

| File                                                               | Change                                                                                                                                           |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `edgequake/Cargo.toml`                                             | Remove `crates/edgequake-pdf` from members; add `edgequake-pdf2md = { version = "0.1", default-features = false }` to `[workspace.dependencies]` |
| `edgequake/crates/edgequake-api/Cargo.toml`                        | Replace `edgequake-pdf` dep with `edgequake-pdf2md`; remove `vision` feature flag (always-on now)                                                |
| `edgequake/crates/edgequake-api/src/processor.rs`                  | Replace all `edgequake_pdf::*` usage with `edgequake_pdf2md::*`; write bytes to tempfile; build `ConversionConfig`; call `convert()`             |
| `edgequake/crates/edgequake-api/src/pipeline_progress_callback.rs` | Remove `use edgequake_pdf::ProgressCallback`; define a local `PdfProgressCallback` trait or remove trait entirely                                |
| `edgequake/crates/edgequake-core/src/types/multitenancy.rs`        | Add `vision_provider` / `vision_model` to `Workspace`, `CreateWorkspaceRequest`, `UpdateWorkspaceRequest`                                        |
| `edgequake/crates/edgequake-api/src/handlers/workspaces_types.rs`  | Mirror the field additions; update OpenAPI `ToSchema`                                                                                            |
| `Makefile`                                                         | Add `EDGEQUAKE_VISION_PROVIDER` and `EDGEQUAKE_VISION_MODEL` to env export blocks                                                                |
| `docs/*.md` / `edgequake_webui/...`                                | Document vision config in workspace settings                                                                                                     |
| `legacy/edgequake-pdf/`                                            | Move (not delete) old crate here                                                                                                                 |

---

## 9. Roadblocks & Mitigations

### RB-1: `convert()` takes a file path, not bytes

**Problem:** The API stores PDF bytes in PostgreSQL (`pdf.pdf_data: Vec<u8>`).
`edgequake_pdf2md::convert()` requires a filesystem path.

**Resolution:** Tracked in [edgequake-pdf2md#2](https://github.com/raphaelmansuy/edgequake-pdf2md/issues/2) —
`convert_from_bytes()` API requested so the library manages the tempfile internally.

**Interim mitigation (until #2 ships):** Write bytes to a `tempfile::NamedTempFile` and pass the path.

```rust
let mut tmp = tempfile::NamedTempFile::new()
    .map_err(|e| TaskError::Processing(format!("tempfile: {e}")))?;
tmp.write_all(&pdf.pdf_data)?;
let tmp_path = tmp.path().to_str().unwrap().to_string();
let output = edgequake_pdf2md::convert(&tmp_path, &config).await?;
// tmp dropped here → file deleted (RAII)
```

### RB-2: Lost per-page progress callbacks

**Problem:** The old `ProgressCallback` trait notified on each page. The new
crate has no such hook.

**Resolution:** Tracked in [edgequake-pdf2md#1](https://github.com/raphaelmansuy/edgequake-pdf2md/issues/1) —
`ConversionProgressCallback` trait specification filed with full interface design.

**Interim mitigation (until #1 ships):** Emit a single "converting" stage at start,
"completed" at end. UX regression — users see a spinner — but functionally correct.

### RB-3: `vision` Cargo feature removal

**Problem:** `edgequake-api/Cargo.toml` exposes `features = ["vision"]` which
enables `edgequake-pdf/vision`. Downstream users may reference this feature.

**Mitigation:** Keep the `vision` feature stub in `edgequake-api/Cargo.toml` but
make it a no-op (empty deps list). Log a deprecation warning at startup when
the feature is set.

### RB-4: Workspace DB migration

**Problem:** Adding `vision_provider` and `vision_model` to the `Workspace`
struct requires a DB migration for the PostgreSQL backend.

**Mitigation:** Both fields are `Option<String>` on the Rust side. The DB
migration adds two nullable `VARCHAR` columns with no default; old rows return
`NULL` which Rust maps to `None`, which falls back to env-var defaults.

### RB-5: pdfium library path

**Problem:** `edgequake-pdf2md` still requires libpdfium to be present at
runtime. The old crate bundled it in `lib/lib/`. The new crate expects
`DYLD_LIBRARY_PATH` (macOS) or `LD_LIBRARY_PATH` (Linux) OR
`PDFIUM_DYNAMIC_LIB_PATH`.

**Mitigation:** The Makefile already sets `PDFIUM_DYNAMIC_LIB_PATH` from the
old crate path. After moving to `legacy/`, update to point at the legacy path
or download fresh via the new crate's setup script. Add a health-check at
server startup that warns when pdfium is not found.

### RB-6: edgequake-pdf was a workspace member; removing it breaks the build

**Problem:** Removing `crates/edgequake-pdf` from the workspace members list
means `cargo build` no longer compiles it. But `edgequake-api/Cargo.toml`
referenced it as `path = "../edgequake-pdf"`.

**Mitigation:** Update `edgequake-api/Cargo.toml` to use
`edgequake-pdf2md = { version = "0.1", default-features = false }` instead.
The legacy crate stays on disk in `legacy/` but is no longer in the workspace
members list.

---

## 10. Testing Plan

| Test                                            | How                                                                            |
| ----------------------------------------------- | ------------------------------------------------------------------------------ |
| Unit: tempfile PDF conversion                   | `cargo test -p edgequake-api --lib` with a small test PDF in `test-data/`      |
| Integration: full PDF upload → markdown → graph | `make test-e2e` or manual upload via WebUI                                     |
| Vision model override                           | Set `EDGEQUAKE_VISION_MODEL=gpt-4.1-mini`, upload PDF, verify markdown quality |
| Workspace-level override                        | Create workspace with `vision_model: "gpt-4.1"`, upload PDF                    |
| Fallback: no API key                            | Verify error is surfaced clearly in task status                                |
| pdfium missing                                  | Verify startup warning and graceful error on upload                            |

---

## 11. SDK & WebUI Changes

### REST API — Workspace endpoints

```json
// POST /v1/workspaces  — CreateWorkspaceRequest
{
  "name": "my-workspace",
  "vision_provider": "openai", // NEW optional
  "vision_model": "gpt-4.1-nano" // NEW optional
}
```

### WebUI Settings Page

Add **Vision Model** section in workspace settings:

- Provider dropdown: openai / anthropic / gemini / ollama
- Model text input (or predefined list)
- Cost estimate badge (e.g., "~$0.02 per 50-page document")

### Makefile quick-start block

```makefile
export EDGEQUAKE_VISION_PROVIDER ?= openai
export EDGEQUAKE_VISION_MODEL    ?= gpt-4.1-nano
```

---

## 12. Migration Checklist

> **Gate:** Start implementation only after `edgequake-pdf2md` issues #1 and #2 are
> released. All items below are pre-work or post-library tasks.

### Pre-work (can be done now)

- [x] Specification written (`specs/pdf-vision/SPEC.md`)
- [x] GitHub issues filed on `edgequake-pdf2md` repo:
  - [#1 ConversionProgressCallback](https://github.com/raphaelmansuy/edgequake-pdf2md/issues/1)
  - [#2 convert_from_bytes API](https://github.com/raphaelmansuy/edgequake-pdf2md/issues/2)
  - [#3 LLMProvider injection docs](https://github.com/raphaelmansuy/edgequake-pdf2md/issues/3)
  - [#4 Error ergonomics](https://github.com/raphaelmansuy/edgequake-pdf2md/issues/4)
- [x] Workspace vision config fields designed (multitenancy types — stored in metadata JSON, no separate columns needed)
- [ ] WebUI settings wireframe for vision provider/model

### Gated on edgequake-pdf2md v0.2

- [x] Move `edgequake/crates/edgequake-pdf` → `legacy/edgequake-pdf`
- [x] Remove `crates/edgequake-pdf` from workspace members
- [x] Pin `edgequake-pdf2md = "0.2"` in workspace deps
- [x] Update `edgequake-api/Cargo.toml`
- [x] Update `processor.rs` (`convert_from_bytes` + `ConversionConfig` + `progress_callback`)
- [x] Update `pipeline_progress_callback.rs` (implement `ConversionProgressCallback`)
- [x] Add `vision_provider` / `vision_model` to multitenancy types
- [x] Add `vision_provider` / `vision_model` to API DTOs
- [x] Add `vision_provider` / `vision_model` to workspace handler
- [x] Vision config stored in metadata JSON (same pattern as LLM/embedding config; no DB schema migration required)
- [x] Update Makefile env vars (`EDGEQUAKE_VISION_PROVIDER`, `EDGEQUAKE_VISION_MODEL`)
- [ ] Update `.env.example`
- [x] `cargo build` passes with zero errors
- [x] `cargo test -p edgequake-api --lib` passes (475/475)
- [ ] End-to-end PDF upload test passes

---

## 13. Appendix: edgequake-pdf2md Supported Providers

| Provider  | Model                    | $/1M in | $/1M out | Vision |
| --------- | ------------------------ | ------- | -------- | ------ |
| openai    | gpt-4.1-nano (default)   | $0.10   | $0.40    | ✓      |
| openai    | gpt-4.1-mini             | $0.40   | $1.60    | ✓      |
| openai    | gpt-4.1                  | $2.00   | $8.00    | ✓      |
| anthropic | claude-sonnet-4-20250514 | $3.00   | $15.00   | ✓      |
| anthropic | claude-haiku-4-20250514  | $0.80   | $4.00    | ✓      |
| gemini    | gemini-2.0-flash         | $0.10   | $0.40    | ✓      |
| gemini    | gemini-2.5-pro           | $1.25   | $10.00   | ✓      |
| ollama    | llava, llama3.2-vision   | free    | free     | ✓      |

A 50-page document costs ~$0.02 with gpt-4.1-nano.

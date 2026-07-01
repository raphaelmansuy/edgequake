# SPEC-036 — First Principles

---

## 1. What problem are we solving?

EdgeQuake is a **consumer** of three external Rust crates. Today it **cannot be built by a third party** from the public repo alone because:

```toml
# edgequake/Cargo.toml (current)
edgequake-pdf2md = { path = "../../edgequake-pdf2md", … }
[patch.crates-io]
edgequake-llm = { path = "../../edgequake-llm" }
edgequake-pdf2md = { path = "../../edgequake-pdf2md" }
```

**First principle:** A reproducible release artifact must resolve all dependencies from **immutable, publicly addressable coordinates** (crates.io version pins + committed `Cargo.lock`).

---

## 2. Invariants

| # | Invariant | Violation symptom |
|---|-----------|-------------------|
| I1 | Every EdgeQuake feature used at runtime must exist in the **published** API of its dependency version | `cargo build` fails off-patch |
| I2 | Publish order follows **dependency DAG** (dependents after dependencies) | `cargo publish` fails: unpublished dep |
| I3 | Tag `vX.Y.Z` == `Cargo.toml` version == CHANGELOG section | CI preflight rejects tag |
| I4 | `Cargo.lock` committed after pin change; CI uses `--locked` | Nondeterministic CI |
| I5 | semver: EdgeQuake pin must match **minimum** API it calls | Silent API drift |

---

## 3. What EdgeQuake actually needs (minimal API surface)

### edgequake-llm

| Symbol | Consumer | Min version |
|--------|----------|-------------|
| `create_production_reranker` | `edgequake-query/src/bootstrap.rs` | **0.6.26** |
| `BiEncoderReranker` (transitive) | `contract_bootstrap_reranker_env.rs` | **0.6.26** |
| `ProviderFactory`, `MockProvider`, traits | 40+ modules | ≤ 0.6.23 |
| `Reranker`, `BM25Reranker` | query engine | ≤ 0.6.20 |

**Conclusion:** `0.6.26` is the **hard floor**, not optional.

### edgequake-pdf2md

| Symbol | Consumer | Min version |
|--------|----------|-------------|
| `convert_from_bytes` | `edgequake-pdf/src/backend/vision.rs` | ≤ 0.7.0 |
| `ConversionConfig::builder()` + checkpoint/progress | same + `pipeline_progress_callback.rs` | ≤ 0.7.0 |
| `ConversionProgressCallback` | `edgequake-api/src/pipeline_progress_callback.rs` | ≤ 0.7.0 |
| `FileCheckpointStore` | `vision.rs` | ≤ 0.7.0 |
| Feature `bundled` | `edgequake-api/Cargo.toml` vision feature | ≤ 0.4.0 |

**Conclusion:** crates.io **0.9.2 satisfies all callsites**. Local path is not functionally required.

### edgeparse-core

| Symbol | Consumer | Min version |
|--------|----------|-------------|
| `convert_bytes`, `ProcessingConfig`, `TableMethod` | `edgequake-pdf/src/backend/edgeparse.rs` | ≤ 0.2.0 |
| `output::markdown`, `PdfDocument` | page-marker injection (SPEC-032 W-09) | ≤ 0.2.0 |

**Conclusion:** crates.io **0.2.5** is fine; no unpublished local changes needed.

---

## 4. Why path patches existed (historical, not architectural)

| Patch | Comment in Cargo.toml | Reality today |
|-------|----------------------|---------------|
| `edgequake-llm` | "until 0.6.26 publishes" | Accurate — 0.6.26 still local |
| `edgequake-pdf2md` | (no comment) | 0.9.2 published 2026-05-06; patch is stale |
| `edgeparse-core` | none | Never patched; always registry |

---

## 5. Edge cases

### E1 — Cargo unifies `edgequake-llm` across workspace

EdgeQuake and `edgequake-pdf2md` both depend on `edgequake-llm`. pdf2md declares `0.6.20` (^0.6.20). EdgeQuake will pin `0.6.26`. Cargo resolves **one** version (0.6.26). **No pdf2md republish required** unless we want the declared dep updated for documentation.

### E2 — pdfium bundled feature & CI

`edgequake-pdf2md` with `features = ["bundled"]` embeds pdfium at compile time. crates.io 0.9.2 artifact includes `build.rs` + `pdfium-auto`. EdgeQuake CI/macOS dev must have network or cache for first build — same as today with path dep.

### E3 — rust-version skew

| Crate | `rust-version` |
|-------|----------------|
| edgequake | 1.95 |
| edgequake-llm | 1.95 |
| edgequake-pdf2md | 1.91 |
| edgeparse-core | 1.85 |

EdgeQuake MSRV dominates. No conflict.

### E4 — `[patch.crates-io]` hides publish gaps

Developers on the monorepo workspace (`edgequake.code-workspace`) see green builds while crates.io consumers would fail. **Removing patches is the verification gate.**

### E5 — Transitive pdf-cos / pdfium-auto

- `edgeparse-core` → `pdf-cos@0.39.0` (published with edgeparse tag)
- `edgequake-pdf2md` → `pdfium-auto@0.3` (published with pdf2md tag)

Publish workflows already handle ordering + 30–45 s index wait.

### E6 — Reranker E2E without live API keys

`contract_bootstrap_reranker_env` uses `MockProvider` embedding + `EDGEQUAKE_RERANKER=cross_encoder`. Must pass with registry `0.6.26` — proves factory wiring, not HTTP reranker.

### E7 — Vision PDF E2E needs Ollama or mock

Multimodal PDF tests may skip without Ollama. Gate: unit/contract tests mandatory; live E2E optional in CI matrix.

---

## 6. Decision

**Do not** republish `edgeparse-core` or `edgequake-pdf2md` unless a functional delta exists.  
**Do** publish `edgequake-llm@0.6.26` (functional delta).  
**Do** optionally cut `edgequake-pdf2md@0.9.3` as a **dependency-hygiene** release bumping declared `edgequake-llm` to `0.6.26` — low risk, improves docs.rs dependency graph clarity.

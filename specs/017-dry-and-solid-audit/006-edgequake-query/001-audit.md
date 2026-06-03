# edgequake-query — DRY & SOLID Audit

> **STATUS: IN-SCOPE REMEDIATION COMPLETE** (137 lines on disk). If your editor shows ~93 lines, you are viewing **git HEAD** — reload file or `git diff` this path.

**Crate path:** `edgequake/crates/edgequake-query`  
**LOC:** ~10,105 (src) | `sota_engine/` ~2,560 LOC  
**Last verified:** 2026-06-03T08:28:24Z — `run_query_e2e.sh` PASS (`001-test-run.log`); contract 5/5, chunk_ranking 10/10, API path 2/2, lib 96; Playwright PNGs `01–02`; workspace build via runner

---

## Executive Summary

**In-scope SPEC-017 query remediation: DONE.** API + Ollama use **`SOTAQueryEngine`** and **`run_query_pipeline`**. QUERY-DRY-002/004 proven (`run_query_e2e.sh`, contract tests, Playwright `01–02.png`). Triple batch embed when keywords off fixes Local chunk ranking.

**Next phase (not this audit):** core orchestrator → SOTA, delete bench `strategies/`, unify types with core. Legacy `QueryEngine` still in `AppState` but unused by production paths.

---

## Remediation Status

| ID | P | Item | Status | Evidence |
|----|---|------|--------|----------|
| QUERY-DRY-002 | P0 | SOTA entry pipeline 3× copy | ✅ | `query_pipeline.rs` + thin `query_entry/*` |
| QUERY-DRY-004 | P1 | `query_modes` vs `vector_queries` duplicate | ✅ | Delegates; hybrid unified |
| QUERY-SOLID-L-001 | P0 | Ollama + API query path on SOTA | ✅ | `003`, `006-api-production-path-contract` (2 tests) |
| QUERY-embed-001 | P1 | Single-vector reuse broke Local ranking | ✅ | `005-embedding-triple-batch-proof.md` |
| QUERY-E2E-UI | P2 | Playwright query route proof | ✅ | `004-playwright-query-ui-proof.md`, PNGs `01–02` |
| QUERY-DRY-003 | P1 | Dead `strategies/` | 🟡 | `@deprecated`; bench-only |
| QUERY-DRY-001 | P0 | Triple stack with core | 🟡 | API on SOTA; core cycle open |
| QUERY-DRY-005–008 | P1–P2 | Keywords, types, tenant | ⬜ | Next phase |

---

## DRY Violations (updated)

| ID | P | Violation | Status | Remediation |
|----|---|-----------|--------|-------------|
| QUERY-DRY-001 | **P0** | Triple query stack with core | 🟡 | API + Ollama on SOTA |
| QUERY-DRY-002 | **P0** | SOTA entry pipeline 3× copy | ✅ | `run_query_pipeline()` |
| QUERY-DRY-003 | **P1** | Dead `strategies/` | 🟡 | Delete after bench migration |
| QUERY-DRY-004 | **P1** | Default vs workspace retrieval | ✅ | `query_modes` delegates |
| QUERY-DRY-005–008 | P1–P2 | Keywords, tenant, types | ⬜ | Planned |
| QUERY-DRY-009 | **P3** | Unused `edgequake-core` dep | ✅ | Not in `Cargo.toml` |

---

## SOLID Violations (updated)

| ID | P | Principle | Status | Notes |
|----|---|-----------|--------|-------|
| QUERY-SOLID-S-001 | P1 | SRP — god surface | 🟡 | Pipeline split; rerank/validate still mixed |
| QUERY-SOLID-O-001 | P1 | OCP — new mode | 🟡 | Centralized in `vector_queries` |
| QUERY-SOLID-L-001 | P0 | LSP — dual engines | ✅ | Ollama + API on SOTA |
| QUERY-SOLID-L-002 | P0 | Core orchestrator cycle | ⬜ | Shared types / core move |
| QUERY-SOLID-I-001 | P2 | `QueryStrategy` unused | 🟡 | Bench-only |

---

## sota_engine Structure (actual)

```text
sota_engine/                    ~2,560 LOC
├── mod.rs              ~570   (+ triple-embed when no keywords)
├── query_modes.rs      ~246   delegates → vector_queries
├── vector_queries.rs   ~585   canonical retrieval
├── query_entry/
│   ├── query_pipeline.rs  ~380
│   ├── query_basic.rs     ~96
│   ├── query_stream.rs    ~133
│   └── query_workspace.rs ~69
├── reranking.rs        ~228
└── prompt.rs           ~284
```

---

## Brutal Assessment

### Proven (code is law)

```bash
./specs/017-dry-and-solid-audit/006-edgequake-query/e2e/run_query_e2e.sh
cd edgequake && cargo check --workspace
```

| Claim | Proof | Gap |
|-------|-------|-----|
| Single pipeline | `spec017_query_pipeline_contract` 5/5 | ✅ |
| Default = workspace retrieval | parity tests | ✅ |
| Chunk score ranking (Local) | `e2e_sota_engine chunk_ranking` 10/10 | ✅ (after embed fix) |
| API + Ollama use SOTA | `spec017_query_production_path_contract` 2/2 | ✅ |
| Workspace compiles | `cargo check --workspace` | ✅ |
| Playwright query UI | `spec017-query-pipeline.spec.ts` + PNGs `01–02` | ✅ |

### On-disk E2E inventory

| Path | Present |
|------|---------|
| `e2e/run_query_e2e.sh` | ✅ |
| `e2e/run_playwright_proof.sh` | ✅ |
| `e2e/001-test-run.log` | ✅ |
| `e2e/001`–`006` proof narratives | ✅ |
| `e2e/screenshots/01–02.png` | ✅ (2026-06-03 16:19 UTC, `run_playwright_proof.sh`) |

### Honest gaps

1. **Legacy `QueryEngine`** in `AppState` — remove when core migrates.
2. **`strategies/`** ~900 LOC — bench-only.
3. **Core ↔ query types** — QUERY-DRY-001/008 open.
4. **Playwright PNGs** — ✅ captured via `run_playwright_proof.sh`.

---

## Next Steps

### P0 — core
1. Break core↔query cycle; route orchestrator through SOTA.
2. Remove legacy `QueryEngine` from `AppState` once core path migrates.

### P1 — cleanup
3. Delete or isolate `strategies/` for benches.
4. Split `validate_keywords` from `reranking.rs`.
5. ~~Run Playwright~~ — done (`01–02.png` in `e2e/screenshots/`).

### P2/P3
6. QUERY-DRY-005 keyword module unification with core.
7. QUERY-DRY-006 shared `matches_tenant` helper.

---

## Verification

```bash
./specs/017-dry-and-solid-audit/006-edgequake-query/e2e/run_query_e2e.sh
cargo test -p edgequake-query --test spec017_query_pipeline_contract
rg 'query_engine' edgequake/crates/edgequake-api/src/handlers/ollama/
```

**Acceptance:** No `query_engine` in Ollama handlers; contract 5/5; `chunk_ranking` 10/10.

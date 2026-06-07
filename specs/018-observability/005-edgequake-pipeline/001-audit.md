# edgequake-pipeline — Observability Audit

**Path:** `edgequake/crates/edgequake-pipeline`  
**Tracing macros (src):** ~53  
**Role:** Chunking, LLM extraction, merging, parsing

---

## Executive Summary

Pipeline is the **noisiest domain crate** at DEBUG — parser modules (`prompts/parser/*`) account for ~20+ debug calls. Production default `RUST_LOG=edgequake_pipeline=debug` in `main.rs` will flood logs.

Extraction (`extractor/sota.rs`) uses appropriate **warn/error** for LLM parse failures.

---

## Findings

| ID | P | Finding | Evidence | Remediation |
|----|---|---------|----------|-------------|
| PIPE-OBS-001 | P1 | No stage timing spans | `pipeline/processing.rs` | `info!(stage, duration_ms)` per stage |
| PIPE-OBS-002 | P2 | Parser debug verbosity | `json_parser.rs`, `mod.rs` | TRACE level for token-level |
| PIPE-OBS-003 | P2 | `error.rs` uses warn once | Central error type | Map retriable vs fatal at WARN/ERROR |
| PIPE-OBS-004 | P2 | Cache hits at debug | `cache.rs` | OK — keep DEBUG |
| PIPE-OBS-005 | P3 | No metrics for extraction tokens | — | Counter in Phase 3 roadmap |

---

## Pipeline Stage Observability (target)

```
  upload ──▶ chunk ──▶ embed ──▶ extract ──▶ merge ──▶ graph
              │         │          │           │
              └─ span: pipeline.stage with duration_ms each
```

---

## Log Quality Examples

| Good | `extractor/sota.rs` — structured warn on parse failure |
| Bad | Free-text `info!` without `document_id` in some merger paths |

**P0 graph integrity bugs (017 audit) are separate — but observability must log normalizer path used.**

---

## Verify

```bash
RUST_LOG=edgequake_pipeline=debug cargo test -p edgequake-pipeline --lib 2>&1 | head -50
```

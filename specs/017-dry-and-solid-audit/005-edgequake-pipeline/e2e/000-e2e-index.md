# E2E Proof Index — SPEC-017 edgequake-pipeline

**Last verified:** 2026-06-04 10:04 UTC

| # | File | Scope |
|---|------|-------|
| 001 | `001-p0-normalizer-chunker-proof.md` | PIPE-DRY-001/002, PIPE-SOLID-L-001/002 |
| 002 | `002-json-dry-consolidation-proof.md` | PIPE-DRY-003/004/005 |
| 003 | `003-playwright-documents-proof.md` | Documents UI shell (screenshots 01–02) |
| 004 | `004-chunker-tests-extraction-proof.md` | PIPE-DRY-009, chunker contract |
| 005 | `005-p2-extractor-progress-parser-proof.md` | Extractor split, StageStatus, parser registry |
| 006 | `006-pipeline-config-schema-proof.md` | PIPE-SOLID-I-001, ConfigurableEntitySchema |
| 007 | `007-stage-bridge-proof.md` | PIPE-DRY-006 stage mappings |
| 008 | `008-full-pipeline-proof.md` | **Full pipeline** — Rust + sync/async/PDF API + Playwright (screenshots 01–07) |

**Runners:**

```bash
./e2e/run_pipeline_e2e.sh              # Rust contract + lib + clippy + fmt
./e2e/run_pipeline_e2e.sh --playwright # + Playwright (6 tests)
./e2e/run_playwright_proof.sh            # Playwright only (6 tests)
```

**Log:** `001-test-run.log`

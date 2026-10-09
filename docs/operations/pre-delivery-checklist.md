---
title: "Pre-delivery checklist"
description: "Local and CI gates to run before tagging a release."
---

### Pre-delivery checklist (v0.19+)

Run these **before** tagging a release. Prefer Makefile targets — they set required env vars.

```bash
# Fast local gates (mirrors CI first principles: fail cheap → compile once → proofs)
make ops17-smoke                # PG extension pin SSOT (pg16/17/18)
make spec046-acc                # SPEC-046 ACC + AccReport JSON
make release-gates              # fmt + workspace clippy + SPEC-006/018 + WebUI + version parity
make test-e2e-lint              # Playwright flake anti-patterns
# SPEC-001 LightRAG Acc (local mandatory before tag — not in CI / release_gates.sh):
make bench001-doctor
make bench                      # EQ vs LightRAG Acc n=200 + publish/latest
# Optional UI-only (no backend): make test-e2e-ui
```

| Gate | What it proves | CI workflow |
|------|----------------|-------------|
| Migration checksum | Immutable SQL lockfile | `CI` → migration-checksum-guard |
| fmt + clippy + lib tests | Code quality | `CI` → check / test (nextest) |
| SPEC-006 / SPEC-018 | Resource + observability proofs | `CI` + `Release Gates` |
| Invariants + test floor | Reliability floor (≥870 lib) | `Test Quality Gates` |
| SPEC-046 ACC | Hybrid RAG science ACC | `SPEC-046 ACC` |
| SPEC-001 LightRAG Acc | EQ vs LightRAG GraphRAG-Bench Acc (n=200) | **Local only** (`make bench`) |
| OPS-17 pins | pgvector/AGE pin matrix | `PostgreSQL Matrix Nightly` |

**CI speed principles** (see `.github/workflows/ci.yml`): shared cargo cache across jobs, `CARGO_INCREMENTAL=0` + sparse index, `--locked`, cancel-in-progress, no duplicate workspace lib suite in sibling workflows, release gates skip per-crate clippy / lib re-run when CI already owns them.

See [AGENTS.md](../../AGENTS.md) for the full developer workflow and [Release & CD](../operations/release-and-cd.md) for the release process.


# SPEC-023 — EdgeQuake Ingestion & Query Brutal Audit

> **Date**: 2026-06-27  
> **Method**: Code Is Law — every claim anchored to `edgequake/crates/`  
> **Baseline**: Post SPEC-022 (P-H1–P-H7 shipped)  
> **Audience**: Engineers shipping production RAG; not marketing

---

## What this spec is

A multi-lens, first-principles audit of EdgeQuake's **ingestion** and **query** pipelines. Each document is cross-referenced by finding ID (`RC-023-*`). Grades reflect **production readiness today**, not roadmap intent.

---

## Document map

| # | File | Purpose |
|---|------|---------|
| 00 | [00-executive-brutal-audit.md](./00-executive-brutal-audit.md) | Grades, ship verdict, top findings |
| 01 | [01-ingestion-first-principles.md](./01-ingestion-first-principles.md) | Ingestion architecture, saga, paths |
| 02 | [02-query-first-principles.md](./02-query-first-principles.md) | Query modes, retrieval, reranking |
| 03 | [03-eight-lens-audit.md](./03-eight-lens-audit.md) | 8 expert lenses + cross-ref matrix |
| 04 | [04-cross-reference-index.md](./04-cross-reference-index.md) | RC-023 finding registry |
| 05 | [05-improvement-plan.md](./05-improvement-plan.md) | Phased plan I1–I10 |

---

## Related specs

| Spec | Relationship |
|------|--------------|
| [022-edgequake-study](../022-edgequake-study/) | Predecessor; closed RC-022-1–6 |
| [021-storage-study](../021-storage-study/) | Storage contracts, pipeline diagrams |
| [017-dry-and-solid-audit](../017-dry-and-solid-audit/) | DRY/SOLID baseline |

---

## How to re-verify (post SPEC-023)

```bash
# I1 injection
cargo test -p edgequake-api --test e2e_spec023_injection_persister

# I3 eval + I5 RRF + I2 docs contract
cargo test -p edgequake-query --test rag_benchmark_recall --test contract_rrf_fusion --test contract_global_mode_semantics

# Persister SSOT
cargo test -p edgequake-pipeline --test contract_ingestion_persistence
cargo test -p edgequake-api --test e2e_spec022_file_upload_persister
```

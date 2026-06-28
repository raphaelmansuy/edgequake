# Phase 2 — Ingestion Parity (4 weeks)

**Parent:** [001-improvement-plan.md](../001-improvement-plan.md)  
**Cross-ref:** [003 Ingestion](../../003-ingestion/001-ingestion-comparison.md) · [007 Rust/SOLID/DRY](../../008-expert-lenses/007-rust-solid-dry.md) · [004 SOTA RAG Jun 2026](../../008-expert-lenses/004-sota-rag-jun2026.md)

**Date:** 2026-06-27  
**Duration:** 4 weeks (Weeks 3–6 of SPEC-026 timeline)  
**Prerequisite:** Phase 1 exit gate (head-to-head eval harness + RAGAS smoke CI green)

---

## Documents

| ID | Topic | File |
|----|-------|------|
| 001 | Master plan (workstreams, architecture, gates) | [001-phase2-ingestion-parity-plan.md](./001-phase2-ingestion-parity-plan.md) |
| 002 | E2E & contract test matrix | [002-e2e-test-matrix.md](./002-e2e-test-matrix.md) |

---

## Scope Summary

| Priority ID | Deliverable | Week |
|-------------|-------------|:----:|
| P-02 | Chunking strategy registry (`fixed` + `recursive`) | 1 |
| P-03 | Markdown IR ingestion (heading-aware splits) | 2 |
| P-04 | Section heading context in extraction prompts | 2 |
| P-08 | LLM role separation (extract / query / summary) | 3 |
| P-11 | Admission saga (staging KV → promote on success) | 3–4 |

**Out of scope for Phase 2:** semantic/vector chunking (P-02 defer), DOCX (Phase 3), multimodal (Phase 4).

---

## Exit Gate

All items in [002-e2e-test-matrix.md](./002-e2e-test-matrix.md) § Exit Criteria must pass before Phase 3 starts.

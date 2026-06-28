# Phase 4 — Scale & Multimodal (6 weeks)

**Parent:** [001-improvement-plan.md](../001-improvement-plan.md)  
**LightRAG reference:** `/Users/raphaelmansuy/Github/03-working/LightRAG`  
**Cross-ref:** [Phase 2](../phase2/README.md) · [007 Rust/SOLID/DRY](../../008-expert-lenses/007-rust-solid-dry.md)

**Date:** 2026-06-27  
**Duration:** 6 weeks (Weeks 11–16 of SPEC-026 timeline)  
**Prerequisite:** Phase 3 exit gate (DOCX + source ID limits)

---

## Documents

| ID | Topic | File |
|----|-------|------|
| 001 | Master plan (workstreams, architecture, gates) | [001-phase4-scale-multimodal-plan.md](./001-phase4-scale-multimodal-plan.md) |
| 002 | E2E & contract test matrix | [002-e2e-test-matrix.md](./002-e2e-test-matrix.md) |

---

## Scope Summary

| Priority ID | Deliverable | Week |
|-------------|-------------|:----:|
| P-07a | Image upload → VLM describe → text inject (LightRAG `analyze` simplified) | 1–2 |
| P-07b | PDF inline images via parser IR sidecars (drawings/tables/equations) | 3–4 |
| P-12 | External task queue (NATS notify + Postgres SSOT) | 4–6 |

**Out of scope for Phase 4:** GraphRAG (Phase 5), full LightRAG sidecar port (MinerU/Docling), SQS (defer to Phase 4.1 if NATS proves pattern).

---

## LightRAG Inspiration Map

| LightRAG module | EdgeQuake Phase 4 port |
|-----------------|------------------------|
| `pipeline.py::_analyze_worker` | Post-parse VLM stage hook in PDF processor |
| `pipeline.py::analyze_multimodal` | `services/vision_content.rs` + `MultimodalProcessOptions` |
| `prompt_multimodal.py` | Structured JSON prompts (`name`, `type`, `description`) |
| `multimodal_context.py` | Surrounding context for Phase 4b (deferred stub) |
| `llm/_vision_utils.py` | Image normalization in `vision_content.rs` |
| `llm_roles.py` VLM role | `LlmRole::Vlm` in `edgequake-core/llm_roles.rs` |
| In-proc `q_parse/q_analyze/q_process` | Keep `ChannelTaskQueue`; add optional NATS bridge |

---

## Exit Gate

All items in [002-e2e-test-matrix.md](./002-e2e-test-matrix.md) § Exit Criteria must pass before Phase 5 starts.

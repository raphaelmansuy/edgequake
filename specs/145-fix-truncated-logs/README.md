# SPEC-145 — Complete Langfuse observation I/O

> **Mission:** Stop silently truncating Langfuse generation / query-root
> Input/Output. Operators, LLM-as-judge, and dataset experiments need the
> **full** model strings (modulo secret redaction and an honest 1 MiB ceiling).
>
> **Method:** Class-based `IoPolicy` in `edgequake-observability` — Complete /
> Structured / Preview — replacing the global 512-byte cap in
> `record_observation_io`.
>
> **Amends:** SPEC-124 LAW-124-18 “truncated I/O” for generation + query-root.
> Does **not** dump secrets, full ingest markdown, or retrieved chunk text.

## One-screen verdict

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│  LAW: Truncation is not privacy. Secrets are denylisted; model I/O is full.  │
│                                                                              │
│  Root cause: OBSERVATION_IO_PREVIEW_CHARS=512 in record_observation_io       │
│  (not Langfuse DB / not Rust SpanLimits value-length).                       │
│                                                                              │
│  Target:                                                                     │
│    Complete   → generate-answer / extract / glean / query root               │
│    Structured → retriever / embed / rerank / ingest stats JSON               │
│    Preview    → ingest document content only (256 bytes)                     │
│    Ceiling    → EDGEQUAKE_LANGFUSE_IO_MAX_BYTES (default 1 MiB) + io_complete│
└──────────────────────────────────────────────────────────────────────────────┘
```

## Document map

```ascii
  README
    → 00-why (5 WHY)
    → 01-first-principles (LAW-145-1..10)
    → 02-cross-ref-matrix
    → 03-code-as-is
    → 04-target-architecture
    → 05-lenses/ (PO, fullstack, DB, UX, MLOps, log expert, AI engineer)
    → 06-ux-ui-spec
    → 07-implementation-plan
    → 08-e2e-test-matrix
    → 09-edge-cases
    → 10-acceptance
    → 11-honest-assessment
```

## Status board

| ID | Item | Status |
|----|------|--------|
| D1 | Doc pack | Done |
| I1 | `IoPolicy` + Complete default for generation/root | Done |
| I2 | Stream path records complete generation I/O | Done |
| I3 | Secret redaction + honest ceiling metadata | Done |
| T1 | InMemory + unit (no keys) | Done |
| T2 | `make spec145-proof` + live Langfuse GET | Done (`make spec145-langfuse-e2e` starts 3.225.5) |
| A1 | Acceptance | Done (checklist in 10-acceptance) |
| I4 | Stream generation span lifetime (LAW-145-9) | Done (`instrument_generation_token_stream`) |
| I5 | `record_structured_io` SSOT (call sites never name IoPolicy) | Done |

## Locked decisions

| Decision | Choice |
|----------|--------|
| Global 512 preview | Removed for Complete class |
| Safety ceiling | 1_048_576 bytes/field; env override |
| Overflow | UTF-8 prefix + `io_complete=false` (never silent mid-token) |
| Ingest body | Stay Preview (256); never full markdown |
| Chunk text | Never in observation I/O (SPEC-125) |
| Spec PII | Synthetic fixtures only (`SYNTH_ORG`, `MARKER_TAIL_COMPLETE`) |

## Cross-spec anchors

| Spec | Relevance |
|------|-----------|
| [SPEC-124](../124-langfuse-support/) | I/O SSOT, dual export, laws amended |
| [SPEC-018](../018-observability/) | Observability SSOT |
| [SPEC-125](../125-better-chunking/) | Counts-only chunking output |
| [`docs/OBSERVABILITY.md`](../../docs/OBSERVABILITY.md) | Operator docs |
| Langfuse skill | `.github/skills/langfuse/` |

## Non-goals

- Langfuse Prompt Management / Playground
- Storing Langfuse secrets in PostgreSQL
- Dumping full retrieved chunks or full ingest markdown
- Changing metadata 200-char filter contract (LAW-124-20)
- Replacing Prometheus / Jaeger

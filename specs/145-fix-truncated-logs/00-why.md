# 00 — Five WHYs

## Problem statement

Langfuse generation **Output** is cut mid-token (synthetic marker like
`corr_` / `MARKER_TAIL_COMPLETE`). Operators cannot judge, replay, or
LLM-as-judge a completion they cannot see. Evidence: Langfuse UI Output
panel ends mid-word; EdgeQuake still produced a longer answer.

**No personal data in this pack** — fixtures use `SYNTH_ORG` and
`MARKER_TAIL_COMPLETE` only.

---

### WHY 1 — Why does Langfuse show a truncated Output?

**Answer:** The string written to `langfuse.observation.output` (and
`gen_ai.completion`) is already short when exported. Langfuse renders what
it receives.

---

### WHY 2 — Why is the exported string short?

**Answer:** Every call to `record_observation_io` runs
`query_preview(text, OBSERVATION_IO_PREVIEW_CHARS)` with
`OBSERVATION_IO_PREVIEW_CHARS = 512`. Completions longer than that lose
their tail before OTLP/ingestion.

---

### WHY 3 — Why was a 512-byte preview the law?

**Answer:** SPEC-124 LAW-124-8 / LAW-124-18 chose “truncated I/O” as a
PII / noise hedge. That conflates **length** with **secret control**.
Langfuse best practices require meaningful full input/output for
evaluators and dataset experiments.

---

### WHY 4 — Why is this not a Langfuse or OTEL SDK limit?

**Answer:** Langfuse OTLP body default is 512 MiB; oversized fields are
logged (~1 MiB), not hard-rejected for a 512-byte string. Rust
`opentelemetry_sdk` 0.32 `SpanLimits` cap **attribute count**, not value
length. The cut is product code.

---

### WHY 5 — Why do streams look worse?

**Answer:** `query_stream.rs` `llm.stream()` path often never records
generation I/O; only chat/complete fallbacks do. The HTTP root span still
gets `record_query_root_io`, which was also capped at 512. Operators see
a truncated root Output and a generation with empty/partial I/O.

---

## Root cause (one line)

```text
A single 512-byte preview in record_observation_io silently truncates
every Langfuse observation Input/Output — including generation and
query-root answers — before export.
```

```ascii
  LLM answer (full)
       │
       v
  record_observation_io
       │
       v
  query_preview(..., 512)   <-- SSOT TRUNCATION
       │
       v
  OTLP / ingestion → Langfuse UI (cut mid-token)
```

## Cross-refs

- Laws: [01-first-principles.md](01-first-principles.md)
- Code: [03-code-as-is.md](03-code-as-is.md)
- Architecture: [04-target-architecture.md](04-target-architecture.md)

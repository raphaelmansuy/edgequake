# Lens — Log Expert

## Two channels (do not conflate)

```ascii
  RUST_LOG / fmt layer     → operator stdout / file logs (may preview)
  OTEL → Langfuse          → observation Input/Output (Complete for GenAI)
```

`EDGEQUAKE_LOG_SPAN_EVENTS` and EnvFilter (SPEC-083 D-46) bound what reaches
the OTEL bridge for **span noise**, not the Complete I/O payload size once a
span is exported.

## Rules

| Concern | Rule |
|---------|------|
| Error log bodies | May stay short (`truncate_chars` in ingestion errors) — not observation I/O |
| Observation I/O | Complete class; never silent 512 |
| Metadata filters | Stay ≤ 200 chars (LAW-124-20) |
| Dual exporters | Jaeger + Langfuse both see same attributes; ceiling protects memory |

## Anti-patterns

- Using log preview helpers for Langfuse generation I/O.
- Assuming `SpanLimits` truncates string values (it does not in Rust 0.32).

## Cross-refs

- SPEC-018 / SPEC-124 observability lenses
- Architecture: [../04-target-architecture.md](../04-target-architecture.md)

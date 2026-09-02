# 08 — E2E Test Matrix

| ID | Scenario | Layer | Assert |
|----|----------|-------|--------|
| U-145-01 | ASCII fixture > 512 with `MARKER_TAIL_COMPLETE` | unit / InMemory | both dual-write keys contain marker; len ≥ fixture |
| U-145-02 | Multibyte `é` × N + marker | unit / InMemory | UTF-8 valid; marker present; no panic |
| U-145-03 | Preview ingest content | unit | truncated at Preview budget; ellipsis |
| U-145-04 | Structured JSON | unit | unchanged; no ellipsis |
| U-145-05 | Secret `sk-lf-…` / `Bearer ` in text | unit | redacted from I/O |
| U-145-06 | Ceiling MAX+1 | unit | `io_complete=false`; UTF-8 prefix; no panic |
| U-145-07 | Empty / None I/O | unit | no crash; attributes unset or empty |
| U-145-08 | Dual-write equality | InMemory | `langfuse.observation.output` == `gen_ai.completion` |
| C-145-01 | Stream path records I/O | contract grep | `query_stream.rs` production records observation I/O on stream path |
| E-145-01 | Live Langfuse GET | e2e script | observation output contains `MARKER_TAIL_COMPLETE` |
| E-145-02 | OTLP path (3.22+) | e2e optional | same as E-145-01 |
| E-145-03 | Ingestion 3.1 fallback | e2e optional | body input/output still full under ceiling |

## Markers (synthetic)

```text
MARKER_TAIL_COMPLETE=__EQ_IO_COMPLETE_TAIL__
SYNTH_ORG=SYNTH_ORG
```

## Commands

```bash
make spec145-proof
make spec145-langfuse-e2e   # needs local Langfuse + stack
```

## Cross-refs

- Edges: [09-edge-cases.md](09-edge-cases.md)
- Acceptance: [10-acceptance.md](10-acceptance.md)

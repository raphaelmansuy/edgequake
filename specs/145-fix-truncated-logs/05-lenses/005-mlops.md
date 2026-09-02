# Lens — MLOps

## Why complete I/O matters

| Consumer | Needs |
|----------|-------|
| LLM-as-judge | Full generation input/output |
| Dataset experiments | Stable, complete I/O across runs |
| Prompt iteration | Compare full completions, not 512-byte stubs |
| Cost/quality dashboards | Tokens remain LAW-124-12; I/O completeness enables qualitative scores |

## Gates

1. InMemory: fixture with `MARKER_TAIL_COMPLETE` past byte 512 present on both dual-write keys.
2. Live Langfuse: GET observation output contains the same marker.
3. Structured ingest/chunking paths still never dump chunk text (SPEC-125).

## Ops knobs

- `EDGEQUAKE_LANGFUSE_IO_MAX_BYTES` for constrained environments.
- Keep batch export non-blocking (LAW-124-4).

## Cross-refs

- E2E: [../08-e2e-test-matrix.md](../08-e2e-test-matrix.md)
- AI engineer: [007-ai-engineer.md](007-ai-engineer.md)

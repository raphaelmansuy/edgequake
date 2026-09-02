# 07 — Implementation Plan

## Phase A — Spec pack (this directory)

Done when README + laws + lenses + matrices land.

## Phase B1 — IoPolicy SSOT

1. Add `IoPolicy` enum / helpers in `edgequake-observability`.
2. `record_observation_io` defaults to Complete; add `record_observation_io_with_policy`.
3. Fix `query_preview` to byte budgets only.
4. Wire Complete for generation/root; Structured for compact helpers; Preview for ingest content.
5. Secret redaction + `EDGEQUAKE_LANGFUSE_IO_MAX_BYTES` + `io_complete` metadata.

## Phase B2 — Stream path

1. After `llm.stream` assemble, record Complete I/O on the generation span.
2. Contract grep: stream source must call `record_observation_io` (or helper) on the stream success path.

## Phase B3 — Docs / env

1. Update `docs/OBSERVABILITY.md` (no longer “truncated I/O” for GenAI).
2. `.env.example` comment for `EDGEQUAKE_LANGFUSE_IO_MAX_BYTES`.
3. Link SPEC-145 from SPEC-124 README (pointer only).

## Phase C — Proof

1. Unit + InMemory tests (`spec145_*` / extended inmemory).
2. `make spec145-proof`.
3. `scripts/spec145_langfuse_io_e2e.sh` + `make spec145-langfuse-e2e`.

## Order

```ascii
  A docs → B1 IoPolicy → B2 stream → B3 docs/env → C proof
              │              │
              └─ unit ───────┘
```

## Cross-refs

- Architecture: [04-target-architecture.md](04-target-architecture.md)
- Edges: [09-edge-cases.md](09-edge-cases.md)
- E2E: [08-e2e-test-matrix.md](08-e2e-test-matrix.md)

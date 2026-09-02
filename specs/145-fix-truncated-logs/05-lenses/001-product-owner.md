# Lens — Product Owner

## Outcome

An operator opens a Langfuse generation observation and sees the **full**
model answer (and prompt) — enough to score quality, debug RAG, and run
LLM-as-judge — without mid-token cuts.

## Jobs to be done

1. Debug a long Mix / Local answer in Langfuse without guessing the missing tail.
2. Trust that export matches what the product returned to the user.
3. Keep secrets out of traces (keys never appear).
4. Keep ingest traces lean (no full document dump).

## Non-goals (v1)

- Changing Langfuse UI itself.
- Persisting observation bodies in EdgeQuake Postgres.
- Dumping every retrieved chunk into observation I/O.

## Success metrics

| Metric | Gate |
|--------|------|
| Fixture > 512 bytes round-trips InMemory | `make spec145-proof` |
| Live Langfuse GET contains `MARKER_TAIL_COMPLETE` | `make spec145-langfuse-e2e` |
| No secret leakage in I/O | unit redaction tests |
| Ingest still counts-only / preview | existing InMemory contracts |

## Risks

| Risk | Mitigation |
|------|------------|
| Memory / batch size growth | 1 MiB ceiling + honest `io_complete` |
| PII in long answers | Redact secrets; ops retention is Langfuse’s; LAW-145-2 |
| Spec leakage of real names | LAW-145-10 synthetic fixtures only |

## Cross-refs

- Acceptance: [../10-acceptance.md](../10-acceptance.md)
- WHY: [../00-why.md](../00-why.md)

# Lens — AI Engineer

## Generation contract

For every `generation` observation (and query/chat root):

| Field | Requirement |
|-------|-------------|
| Input | Prompt / user query as sent (redacted) |
| Output | Full completion (redacted) |
| Usage | Tokens when provider returns them |
| Cost attrs | Never (LAW-124-12) |

## Stream / cache / glean

- Stream: record after assemble (LAW-145-9).
- Cache hit: if answer is recorded on the span, it must be Complete.
- Gleaning: each `with_llm_generation` keeps its own Complete I/O.

## Fixtures (no PII)

```text
SYNTH_ORG=SYNTH_ORG
MARKER_TAIL_COMPLETE=__EQ_IO_COMPLETE_TAIL__
```

Build answers: `"a".repeat(600) + MARKER_TAIL_COMPLETE` and multibyte
`"é".repeat(400) + MARKER_TAIL_COMPLETE`.

## Cross-refs

- Laws: [../01-first-principles.md](../01-first-principles.md)
- MLOps: [005-mlops.md](005-mlops.md)

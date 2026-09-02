# 01 — First Principles (LAW-145)

## Axioms

| ID | Law | Operational meaning |
|----|-----|---------------------|
| **LAW-145-1** | Completeness of model I/O | Generation Input/Output are byte-identical to the LLM prompt/completion strings, modulo secret redaction (never product-truncate by default) |
| **LAW-145-2** | Truncation ≠ privacy | Secrets are denylisted; length caps are not a privacy strategy |
| **LAW-145-3** | One I/O SSOT | Only `record_observation_io` / typed wrappers set Langfuse I/O keys (keeps LAW-124-16) |
| **LAW-145-4** | Policy by class | `Complete` \| `Structured` \| `Preview` — never one global 512 |
| **LAW-145-5** | UTF-8 honesty | Budgets are **bytes**; never mix `chars().count()` with `utf8_prefix` byte cuts |
| **LAW-145-6** | Honest overflow | If an optional ceiling is set and hit: UTF-8 prefix + `io_complete=false` + `io_bytes`; never silent mid-token |
| **LAW-145-7** | Dual-write stays | `langfuse.observation.*` + `gen_ai.prompt` / `gen_ai.completion` (LAW-124-17) |
| **LAW-145-8** | Unfakable proof | InMemory asserts full fixture; live Langfuse GET asserts `MARKER_TAIL_COMPLETE` |
| **LAW-145-9** | Stream completeness | Token streams record I/O **after** assemble — same as non-stream |
| **LAW-145-10** | No personal data in spec | Synthetic fixtures only (`SYNTH_ORG`, `MARKER_TAIL_COMPLETE`) |
| **LAW-145-11** | Full LLM payload | Generation `input` is the prompt/chat turns sent to the model — not a UI query stub |

## Amendments to SPEC-124

| Prior | SPEC-145 change |
|-------|-----------------|
| LAW-124-8 Explicit span I/O | Unchanged: never dump API keys / full configs / all fn args |
| LAW-124-18 “truncated I/O” for GenAI | **Superseded** for generation + query-root by LAW-145-1 |
| LAW-124-20 metadata 200 chars | Unchanged |
| LAW-124-16 I/O SSOT | Unchanged; `IoPolicy` lives inside the SSOT |

## Anti-patterns

| Anti-pattern | Violates |
|--------------|----------|
| One global `OBSERVATION_IO_PREVIEW_CHARS` for all classes | LAW-145-4 |
| Silent `…` cut without `io_complete` | LAW-145-6 |
| `chars().count()` gate + byte `utf8_prefix` | LAW-145-5 |
| Dump full ingest markdown as observation input | LAW-145-4 Preview |
| Call-site invents its own cap | LAW-145-3 |
| Stream path skips generation I/O | LAW-145-9 |
| Spec examples with real person/org names | LAW-145-10 |

## Env contract (additive)

| Variable | Default | Purpose |
|----------|---------|---------|
| `EDGEQUAKE_LANGFUSE_IO_MAX_BYTES` | `0` (unlimited) | Optional per-field ceiling for Complete class; `0` / unset = never product-truncate |

## Cross-refs

- WHY: [00-why.md](00-why.md)
- Architecture: [04-target-architecture.md](04-target-architecture.md)
- SPEC-124: [../124-langfuse-support/01-first-principles.md](../124-langfuse-support/01-first-principles.md)

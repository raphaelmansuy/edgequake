# 09 — Edge Cases

| ID | Case | Mitigation | Test |
|----|------|------------|------|
| EC-01 | Empty completion | Emit empty string or skip; no panic | U-145-07 |
| EC-02 | Unicode / emoji / combining marks | Byte `utf8_prefix` only | U-145-02 |
| EC-03 | Streaming token-by-token | Record after assemble | C-145-01 / E-145-01 |
| EC-04 | Cache-hit answer | Complete when recorded | unit path |
| EC-05 | Gleaning multi-generation | Each `with_llm_generation` Complete | existing glean + U-145-01 |
| EC-06 | Dual exporter memory | 1 MiB ceiling | U-145-06 |
| EC-07 | 3.1 ingestion JSON size | Same Complete body; error log truncate only | E-145-03 |
| EC-08 | Langfuse HTTP 413 | Log; do not fail user request (LAW-124-4) | ops / existing export |
| EC-09 | Attribute count limit 128 | Set I/O keys early; avoid attribute spam | contract |
| EC-10 | Concurrent spans | Span-local attributes | InMemory |
| EC-11 | `io_complete` metadata 200 cap | Short values `"false"` / digit length | U-145-06 |
| EC-12 | Redaction false positive on prose `sk-` | Prefer key-shaped patterns (`sk-lf-`, `sk-proj-`, Bearer) | U-145-05 |
| EC-13 | Root + child both Complete | Both helpers use Complete | U-145-01 |
| EC-14 | Ingest full markdown temptation | Preview only | U-145-03 + SPEC-125 |
| EC-15 | Mid-token cut at old 512 | Gone for Complete | U-145-01 |

## Cross-refs

- Laws: [01-first-principles.md](01-first-principles.md)
- E2E: [08-e2e-test-matrix.md](08-e2e-test-matrix.md)

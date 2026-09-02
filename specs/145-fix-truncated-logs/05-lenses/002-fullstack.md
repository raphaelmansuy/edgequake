# Lens — Full Stack Developer

## SOLID / DRY

| Principle | Application |
|-----------|-------------|
| S | `IoPolicy` owns clamp/redact; call sites only choose class |
| O | New classes without rewriting exporters |
| L | Typed wrappers (`record_query_root_io`) preserve Complete |
| I | Compact helpers stay Structured; never forced through Preview |
| D | Query/API depend on observability helpers, not byte constants |
| DRY | One SSOT — no per-crate 512 copies |

## Touch points

1. `edgequake-observability`: `IoPolicy`, `record_observation_io`, `query_preview` byte fix, env ceiling.
2. `edgequake-query` `query_stream.rs`: record Complete I/O after stream assemble.
3. Docs / `.env.example` / Makefile targets.
4. Tests: `inmemory_otel_tests.rs`, `rag_span` unit, contract greps.

## Do not

- Invent caps in pipeline/query/api.
- Rewrite SPEC-124 docs in place — link SPEC-145.
- Change ingestion envelope mapping (LAW-124-23).

## Cross-refs

- Architecture: [../04-target-architecture.md](../04-target-architecture.md)
- Plan: [../07-implementation-plan.md](../07-implementation-plan.md)

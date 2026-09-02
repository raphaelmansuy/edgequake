# 02 — Cross-ref Matrix

## Claim → Authority

| Claim | Authority |
|-------|-----------|
| Observation I/O attribute keys | SPEC-124 LAW-124-16/17; `langfuse_attrs.rs` |
| 512 preview as-is | `rag_span.rs` `OBSERVATION_IO_PREVIEW_CHARS` |
| Dual export OTLP + ingestion | SPEC-124 LAW-124-1/23 |
| Tokens yes / cost never | SPEC-124 LAW-124-12 |
| Chunking output counts only | SPEC-125 / InMemory `inmemory_ingest_chunking` |
| Metadata value cap 200 | LAW-124-20; `LANGFUSE_METADATA_VALUE_MAX_CHARS` |
| Langfuse best-practice I/O | https://langfuse.com/docs/observability/best-practices |
| OTEL SpanLimits (count only) | `opentelemetry_sdk` 0.32 `SpanLimits` |
| Langfuse body limit | `LANGFUSE_OTEL_INGESTION_MAX_BODY_BYTES` (default 512 MiB) |

## Code SSOT (as-is → target)

| Concern | As-is | Target |
|---------|-------|--------|
| I/O clamp | Always `query_preview(..., 512)` | `IoPolicy` per class |
| Generation I/O | Truncated via SSOT | Complete (+ redact + ceiling) |
| Query/chat root | Truncated | Complete |
| Retriever / embed / rerank | Truncated unnecessarily | Structured (as-is JSON) |
| Ingest document content | 256 preview then 512 again | Preview 256 only |
| Stream generation I/O | Often missing | Record after assemble |
| Overflow | Silent `…` | `io_complete=false` + `io_bytes` |

## Related specs

| Spec | Relationship |
|------|--------------|
| SPEC-124 | Parent; I/O completeness amendment |
| SPEC-018 | Observability hub |
| SPEC-125 | Must not dump chunk text |
| SPEC-135 | Chunking meta stays structured |
| SPEC-103 | Cache-hit answers still Complete when recorded |

## Cross-refs

- Architecture: [04-target-architecture.md](04-target-architecture.md)
- Plan: [07-implementation-plan.md](07-implementation-plan.md)

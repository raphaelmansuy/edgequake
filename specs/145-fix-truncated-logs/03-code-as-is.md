# 03 — Code as-is

## Choke point

[`edgequake-observability/src/rag_span.rs`](../../edgequake/crates/edgequake-observability/src/rag_span.rs):

```text
OBSERVATION_IO_PREVIEW_CHARS = 512

record_observation_io(input, output)
  → query_preview(inp, 512)
  → dual-write langfuse.observation.input|output + gen_ai.prompt|completion
```

`query_preview` uses `chars().count()` for the gate, then `utf8_prefix` with a
**byte** budget — LAW-145-5 violation waiting to bite multibyte text.

## Call graph

```ascii
  LlmGenerationRecord::record_on_current_span
  with_llm_generation (Ok path)
  record_query_root_io
  record_rag_retrieval_io / record_embedding_io / ingest helpers
  query_pipeline / query_stream / handlers
       │
       └──► record_observation_io  (always 512)
```

## Key files

| File | Role |
|------|------|
| `rag_span.rs` | SSOT helpers + 512 constant |
| `utf8_truncate.rs` | Byte-safe prefix (correct primitive) |
| `langfuse_attrs.rs` | Attribute key SSOT; metadata 200 |
| `langfuse_meta.rs` | Filterable metadata (truncated to 200 — keep) |
| `langfuse_ingestion.rs` | 3.1 bridge; passes I/O through; `truncate_chars` only for **error logs** |
| `subscriber.rs` | Dual exporters; no value-length SpanLimits |
| `query_stream.rs` | Stream branch often skips generation I/O |
| `handlers/query/*`, `handlers/chat/*` | `record_query_root_io` after answer |

## What is already short (not the bug)

Retriever output JSON, embed dim JSON, rerank `{applied,backend}`,
ingest stats, chunking distribution — these stay Structured. The screenshot
bug is **generation / root answer** text.

## Cross-refs

- Target: [04-target-architecture.md](04-target-architecture.md)
- WHY: [00-why.md](00-why.md)

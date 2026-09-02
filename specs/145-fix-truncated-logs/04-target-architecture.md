# 04 — Target Architecture

## IoPolicy SSOT

```ascii
  record_observation_io(text, class)   [or typed wrappers]
       │
       ├── Complete
       │     redact secrets
       │     if len <= MAX_BYTES → emit full
       │     else → utf8_prefix(MAX) + metadata io_complete=false, io_bytes=N
       │     dual-write langfuse.observation.* + gen_ai.*
       │
       ├── Structured
       │     emit JSON as-is (tiny; no 512)
       │
       └── Preview { max_bytes }
             utf8_prefix + ellipsis (ingest content only)
```

## Class matrix

| Class | Policy | Call sites |
|-------|--------|------------|
| Complete | Full + redact + ceiling | `LlmGenerationRecord`, `record_query_root_io`, generation helpers |
| Structured | As-is | `record_embedding_io`, `record_rag_retrieval_io`, rerank JSON, ingest stats, chunking |
| Preview | 256 bytes content | `record_ingest_document_input` only |

## Safety ceiling

- Default: `EDGEQUAKE_LANGFUSE_IO_MAX_BYTES=1048576`
- Protects **EdgeQuake** process / batcher memory — not Langfuse’s DB
- Overflow is **honest** (LAW-145-6)

## Secret redaction (Complete)

Replace / strip substrings before emit:

- `sk-lf-…`, `sk-proj-…`, bare `sk-` key-shaped tokens
- `Bearer ` + token
- Literal `LANGFUSE_SECRET_KEY` value shapes

Do **not** treat entity names or document prose as secrets.

## Stream path

```ascii
  llm.stream() tokens
       │
       v
  assemble full answer (handler / engine)
       │
       ├── generation span: record_observation_io(Complete)
       └── root span: record_query_root_io(Complete)
```

## Non-goals in architecture

- No PostgreSQL column for observation bodies
- No change to metadata 200-char filter values
- No dump of retrieved chunk text

## Cross-refs

- Laws: [01-first-principles.md](01-first-principles.md)
- Plan: [07-implementation-plan.md](07-implementation-plan.md)
- Fullstack lens: [05-lenses/002-fullstack.md](05-lenses/002-fullstack.md)

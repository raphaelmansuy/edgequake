---
title: 'Crate Reference'
description: EdgeQuake is organized into 11 focused Rust crates, each with a single responsibility.
---

## EdgeQuake Crate Architecture

EdgeQuake is organized into **11 focused Rust crates**, each with a single responsibility. This modular design enables independent testing, clear dependency boundaries, and flexible composition.

### Core Crates

| Crate | Purpose |
|-------|---------|
| `edgequake-api` | HTTP entry point (Axum server) |
| `edgequake-core` | Orchestration and public API |
| `edgequake-pipeline` | Document processing pipeline |
| `edgequake-query` | Query engine for knowledge graph |
| `edgequake-storage` | Storage adapters (PostgreSQL + pgvector + AGE; in-memory adapters for tests only) |
| `edgequake-llm` | LLM providers (OpenAI, Anthropic, Mistral, Gemini, Vertex AI, Ollama, LM Studio, OpenRouter, xAI, mock, …) |
| `edgequake-pdf` | PDF extraction and processing |
| `edgequake-graph` | Graph data structures and algorithms |

### Dependency Flow

```
edgequake-api
  └── edgequake-core
        ├── edgequake-pipeline
        │     └── edgequake-pdf
        ├── edgequake-query
        ├── edgequake-storage
        └── edgequake-llm
```

See the [Architecture Overview](/docs/architecture/overview/) for the full architectural picture.

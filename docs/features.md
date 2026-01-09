# EdgeQuake Feature Registry

> Central registry of all features in EdgeQuake RAG system.
> Use FEATXXXX references in code comments for traceability.

**Version**: 1.5.0 | **Last Updated**: 2026-01-09

---

## Namespace Allocation

| Range    | Module/Team        | Description                                     |
| -------- | ------------------ | ----------------------------------------------- |
| FEAT00XX | Core Pipeline      | Document ingestion, chunking, entity extraction |
| FEAT01XX | Query Engine       | Query modes, caching, optimization              |
| FEAT02XX | Graph Storage      | PostgreSQL AGE, traversal, community detection  |
| FEAT03XX | Streaming/Pipeline | Chunking strategies, async processing           |
| FEAT04XX | Conversations/API  | Citations, history, REST endpoints              |
| FEAT05XX | PDF/Lineage        | PDF extraction, document provenance             |
| FEAT06XX | WebUI Core         | React components, stores, state management      |
| FEAT07XX | WebUI API/Utils    | API client, hooks, utilities                    |
| FEAT08XX | Authentication     | API keys, JWT, tenant isolation                 |
| FEAT085X | Cost Management    | Token tracking, cost estimation                 |
| FEAT086X | WebUI Providers    | React context providers                         |
| FEAT087X | Auth UI            | Login, registration, auth components            |
| FEAT09XX | Dashboard          | Analytics, metrics, system monitoring           |
| FEAT10XX | Document Mgmt UI   | Upload, preview, folder management              |

> **Note**: Cross-cutting features (same ID in types, stores, hooks, components) are intentional.
> See [SKILL.md](../.github/skills/doc-traceability-validator/SKILL.md) for validation details.

---

## Quick Reference Index

| Category                                                 | ID Range          | Count |
| -------------------------------------------------------- | ----------------- | ----- |
| [Core RAG Features](#core-rag-features-feat00xx)         | FEAT0001-FEAT0020 | 20    |
| [Query Engine Features](#query-engine-features-feat01xx) | FEAT0101-FEAT0120 | 10    |
| [Storage Features](#storage-features-feat02xx)           | FEAT0201-FEAT0220 | 5     |
| [Pipeline Features](#pipeline-features-feat03xx)         | FEAT0301-FEAT0320 | 4     |
| [API Features](#api-features-feat04xx)                   | FEAT0401-FEAT0420 | 6     |
| [PDF Features](#pdf-features-feat05xx)                   | FEAT0501-FEAT0520 | 5     |
| [WebUI Features](#webui-features-feat06xx)               | FEAT0601-FEAT0620 | 20    |
| [WebUI API Client](#webui-api-client-features-feat07xx)  | FEAT0700-FEAT0734 | 18    |
| [Auth Features](#auth-features-feat08xx)                 | FEAT0801-FEAT0820 | 3     |
| [Advanced PDF Features](#advanced-pdf-features-feat10xx) | FEAT1001-FEAT1025 | 14    |

---

## Core RAG Features (FEAT00XX)

### FEAT0001 - Document Ingestion

| Attribute          | Value                                                                               |
| ------------------ | ----------------------------------------------------------------------------------- |
| **ID**             | FEAT0001                                                                            |
| **Name**           | Document Ingestion                                                                  |
| **Module**         | edgequake-core                                                                      |
| **Status**         | ✅ Stable                                                                           |
| **Code Reference** | [orchestrator.rs#L200](../edgequake/crates/edgequake-core/src/orchestrator.rs#L200) |
| **Description**    | Accept raw text or file content and process through the pipeline                    |
| **Related**        | BR0001, UC0001                                                                      |

### FEAT0002 - Text Chunking

| Attribute          | Value                                                               |
| ------------------ | ------------------------------------------------------------------- |
| **ID**             | FEAT0002                                                            |
| **Name**           | Text Chunking with Overlap                                          |
| **Module**         | edgequake-pipeline                                                  |
| **Status**         | ✅ Stable                                                           |
| **Code Reference** | [chunker.rs](../edgequake/crates/edgequake-pipeline/src/chunker.rs) |
| **Description**    | Split documents into overlapping chunks for LLM context windows     |
| **Related**        | BR0002, FEAT0001                                                    |

### FEAT0003 - Entity Extraction

| Attribute          | Value                                                                   |
| ------------------ | ----------------------------------------------------------------------- |
| **ID**             | FEAT0003                                                                |
| **Name**           | LLM-Based Entity Extraction                                             |
| **Module**         | edgequake-pipeline                                                      |
| **Status**         | ✅ Stable                                                               |
| **Code Reference** | [extractor.rs](../edgequake/crates/edgequake-pipeline/src/extractor.rs) |
| **Description**    | Extract named entities from text using LLM with SOTA tuple format       |
| **Related**        | BR0003, UC0001, FEAT0002                                                |

### FEAT0004 - Relationship Extraction

| Attribute          | Value                                                                   |
| ------------------ | ----------------------------------------------------------------------- |
| **ID**             | FEAT0004                                                                |
| **Name**           | Relationship Extraction                                                 |
| **Module**         | edgequake-pipeline                                                      |
| **Status**         | ✅ Stable                                                               |
| **Code Reference** | [extractor.rs](../edgequake/crates/edgequake-pipeline/src/extractor.rs) |
| **Description**    | Extract relationships between entities using LLM                        |
| **Related**        | FEAT0003, BR0004                                                        |

### FEAT0005 - Knowledge Graph Construction

| Attribute          | Value                                                             |
| ------------------ | ----------------------------------------------------------------- |
| **ID**             | FEAT0005                                                          |
| **Name**           | Knowledge Graph Construction                                      |
| **Module**         | edgequake-pipeline                                                |
| **Status**         | ✅ Stable                                                         |
| **Code Reference** | [merger.rs](../edgequake/crates/edgequake-pipeline/src/merger.rs) |
| **Description**    | Merge extracted entities and relationships into a knowledge graph |
| **Related**        | FEAT0003, FEAT0004, BR0005                                        |

### FEAT0006 - Vector Embedding Generation

| Attribute          | Value                                                        |
| ------------------ | ------------------------------------------------------------ |
| **ID**             | FEAT0006                                                     |
| **Name**           | Vector Embedding Generation                                  |
| **Module**         | edgequake-llm                                                |
| **Status**         | ✅ Stable                                                    |
| **Code Reference** | [traits.rs](../edgequake/crates/edgequake-llm/src/traits.rs) |
| **Description**    | Generate vector embeddings for text chunks and entities      |
| **Related**        | FEAT0002, BR0006                                             |

### FEAT0007 - Multi-Mode Query Execution

| Attribute          | Value                                                          |
| ------------------ | -------------------------------------------------------------- |
| **ID**             | FEAT0007                                                       |
| **Name**           | Multi-Mode Query Execution                                     |
| **Module**         | edgequake-query                                                |
| **Status**         | ✅ Stable                                                      |
| **Code Reference** | [engine.rs](../edgequake/crates/edgequake-query/src/engine.rs) |
| **Description**    | Execute queries using different retrieval strategies           |
| **Related**        | FEAT0101-FEAT0106, UC0201                                      |

### FEAT0008 - Streaming Response Generation

| Attribute          | Value                                                          |
| ------------------ | -------------------------------------------------------------- |
| **ID**             | FEAT0008                                                       |
| **Name**           | SSE Streaming Responses                                        |
| **Module**         | edgequake-api                                                  |
| **Status**         | ✅ Stable                                                      |
| **Code Reference** | [streaming/](../edgequake/crates/edgequake-api/src/streaming/) |
| **Description**    | Stream LLM responses using Server-Sent Events                  |
| **Related**        | BR0007, UC0202                                                 |

### FEAT0009 - Entity Normalization

| Attribute          | Value                                                                       |
| ------------------ | --------------------------------------------------------------------------- |
| **ID**             | FEAT0009                                                                    |
| **Name**           | Entity Name Normalization                                                   |
| **Module**         | edgequake-pipeline                                                          |
| **Status**         | ✅ Stable                                                                   |
| **Code Reference** | [prompts/mod.rs](../edgequake/crates/edgequake-pipeline/src/prompts/mod.rs) |
| **Description**    | Normalize entity names to UPPERCASE_UNDERSCORED format                      |
| **Related**        | BR0003, FEAT0003                                                            |

### FEAT0010 - Description Summarization

| Attribute          | Value                                                                     |
| ------------------ | ------------------------------------------------------------------------- |
| **ID**             | FEAT0010                                                                  |
| **Name**           | Entity Description Summarization                                          |
| **Module**         | edgequake-pipeline                                                        |
| **Status**         | ✅ Stable                                                                 |
| **Code Reference** | [summarizer.rs](../edgequake/crates/edgequake-pipeline/src/summarizer.rs) |
| **Description**    | Summarize and merge entity descriptions using LLM                         |
| **Related**        | FEAT0005, BR0008                                                          |

### FEAT0011 - Lineage Tracking

| Attribute          | Value                                                               |
| ------------------ | ------------------------------------------------------------------- |
| **ID**             | FEAT0011                                                            |
| **Name**           | Document-Chunk-Entity Lineage                                       |
| **Module**         | edgequake-pipeline                                                  |
| **Status**         | ✅ Stable                                                           |
| **Code Reference** | [lineage.rs](../edgequake/crates/edgequake-pipeline/src/lineage.rs) |
| **Description**    | Track origin of entities back to source documents                   |
| **Related**        | UC0101, BR0009                                                      |

### FEAT0012 - Progress Reporting

| Attribute          | Value                                                                 |
| ------------------ | --------------------------------------------------------------------- |
| **ID**             | FEAT0012                                                              |
| **Name**           | Pipeline Progress Reporting                                           |
| **Module**         | edgequake-pipeline                                                    |
| **Status**         | ✅ Stable                                                             |
| **Code Reference** | [progress.rs](../edgequake/crates/edgequake-pipeline/src/progress.rs) |
| **Description**    | Report real-time progress during document processing                  |
| **Related**        | UC0001, FEAT0401                                                      |

### FEAT0013 - Cost Tracking

| Attribute          | Value                                                                             |
| ------------------ | --------------------------------------------------------------------------------- |
| **ID**             | FEAT0013                                                                          |
| **Name**           | LLM Cost Tracking                                                                 |
| **Module**         | edgequake-pipeline                                                                |
| **Status**         | ✅ Stable                                                                         |
| **Code Reference** | [progress.rs#CostTracker](../edgequake/crates/edgequake-pipeline/src/progress.rs) |
| **Description**    | Track token usage and estimated costs for LLM calls                               |
| **Related**        | BR0301, BR0302                                                                    |

### FEAT0014 - LLM Response Caching

| Attribute          | Value                                                      |
| ------------------ | ---------------------------------------------------------- |
| **ID**             | FEAT0014                                                   |
| **Name**           | LLM Response Caching                                       |
| **Module**         | edgequake-llm                                              |
| **Status**         | ✅ Stable                                                  |
| **Code Reference** | [cache.rs](../edgequake/crates/edgequake-llm/src/cache.rs) |
| **Description**    | Cache LLM responses to reduce redundant API calls          |
| **Related**        | BR0302, FEAT0013                                           |

### FEAT0015 - Multi-Tenant Isolation

| Attribute          | Value                                                                         |
| ------------------ | ----------------------------------------------------------------------------- |
| **ID**             | FEAT0015                                                                      |
| **Name**           | Multi-Tenant Data Isolation                                                   |
| **Module**         | edgequake-core                                                                |
| **Status**         | ✅ Stable                                                                     |
| **Code Reference** | [tenant_manager.rs](../edgequake/crates/edgequake-core/src/tenant_manager.rs) |
| **Description**    | Isolate data and operations per tenant                                        |
| **Related**        | BR0201-BR0203, UC0301                                                         |

### FEAT0016 - Workspace Management

| Attribute          | Value                                                                               |
| ------------------ | ----------------------------------------------------------------------------------- |
| **ID**             | FEAT0016                                                                            |
| **Name**           | Workspace CRUD Operations                                                           |
| **Module**         | edgequake-core                                                                      |
| **Status**         | ✅ Stable                                                                           |
| **Code Reference** | [workspace_service.rs](../edgequake/crates/edgequake-core/src/workspace_service.rs) |
| **Description**    | Create, read, update, delete workspaces per tenant                                  |
| **Related**        | UC0301-UC0304, BR0202                                                               |

### FEAT0017 - Conversation Management

| Attribute          | Value                                                                                     |
| ------------------ | ----------------------------------------------------------------------------------------- |
| **ID**             | FEAT0017                                                                                  |
| **Name**           | Conversation Management                                                                   |
| **Module**         | edgequake-core                                                                            |
| **Status**         | ✅ Stable                                                                                 |
| **Code Reference** | [conversation_service.rs](../edgequake/crates/edgequake-core/src/conversation_service.rs) |
| **Description**    | Manage conversation history and context                                                   |
| **Related**        | UC0401-UC0405, FEAT0007                                                                   |

### FEAT0018 - Rate Limiting

| Attribute          | Value                                                                   |
| ------------------ | ----------------------------------------------------------------------- |
| **ID**             | FEAT0018                                                                |
| **Name**           | Per-Tenant Rate Limiting                                                |
| **Module**         | edgequake-rate-limiter                                                  |
| **Status**         | ✅ Stable                                                               |
| **Code Reference** | [limiter.rs](../edgequake/crates/edgequake-rate-limiter/src/limiter.rs) |
| **Description**    | Limit API requests per tenant to prevent abuse                          |
| **Related**        | BR0204, FEAT0015                                                        |

### FEAT0019 - Background Task Processing

| Attribute          | Value                                                          |
| ------------------ | -------------------------------------------------------------- |
| **ID**             | FEAT0019                                                       |
| **Name**           | Background Task Queue                                          |
| **Module**         | edgequake-tasks                                                |
| **Status**         | ✅ Stable                                                      |
| **Code Reference** | [worker.rs](../edgequake/crates/edgequake-tasks/src/worker.rs) |
| **Description**    | Process long-running tasks asynchronously                      |
| **Related**        | FEAT0001, UC0005                                               |

### FEAT0020 - Audit Logging

| Attribute          | Value                                                          |
| ------------------ | -------------------------------------------------------------- |
| **ID**             | FEAT0020                                                       |
| **Name**           | Audit Event Logging                                            |
| **Module**         | edgequake-audit                                                |
| **Status**         | ✅ Stable                                                      |
| **Code Reference** | [logger.rs](../edgequake/crates/edgequake-audit/src/logger.rs) |
| **Description**    | Log security-relevant events for compliance                    |
| **Related**        | BR0205, FEAT0015                                               |

---

## Query Engine Features (FEAT01XX)

### FEAT0101 - Naive Vector Search

| Attribute          | Value                                                                                |
| ------------------ | ------------------------------------------------------------------------------------ |
| **ID**             | FEAT0101                                                                             |
| **Name**           | Naive Vector Similarity Search                                                       |
| **Module**         | edgequake-query                                                                      |
| **Status**         | ✅ Stable                                                                            |
| **Code Reference** | [strategies.rs#NaiveStrategy](../edgequake/crates/edgequake-query/src/strategies.rs) |
| **Description**    | Pure vector similarity search without graph context                                  |
| **Related**        | FEAT0007, UC0201                                                                     |

### FEAT0102 - Local Entity-Centric Search

| Attribute          | Value                                                                                |
| ------------------ | ------------------------------------------------------------------------------------ |
| **ID**             | FEAT0102                                                                             |
| **Name**           | Local Entity-Centric Search                                                          |
| **Module**         | edgequake-query                                                                      |
| **Status**         | ✅ Stable                                                                            |
| **Code Reference** | [strategies.rs#LocalStrategy](../edgequake/crates/edgequake-query/src/strategies.rs) |
| **Description**    | Search centered on entities with local graph context                                 |
| **Related**        | FEAT0007, UC0201, FEAT0005                                                           |

### FEAT0103 - Global Community-Based Search

| Attribute          | Value                                                                                 |
| ------------------ | ------------------------------------------------------------------------------------- |
| **ID**             | FEAT0103                                                                              |
| **Name**           | Global Community-Based Search                                                         |
| **Module**         | edgequake-query                                                                       |
| **Status**         | ✅ Stable                                                                             |
| **Code Reference** | [strategies.rs#GlobalStrategy](../edgequake/crates/edgequake-query/src/strategies.rs) |
| **Description**    | Search using community detection for global context                                   |
| **Related**        | FEAT0007, FEAT0205                                                                    |

### FEAT0104 - Hybrid Search

| Attribute          | Value                                                                                 |
| ------------------ | ------------------------------------------------------------------------------------- |
| **ID**             | FEAT0104                                                                              |
| **Name**           | Hybrid Local+Global Search                                                            |
| **Module**         | edgequake-query                                                                       |
| **Status**         | ✅ Stable                                                                             |
| **Code Reference** | [strategies.rs#HybridStrategy](../edgequake/crates/edgequake-query/src/strategies.rs) |
| **Description**    | Combine local and global search strategies                                            |
| **Related**        | FEAT0102, FEAT0103                                                                    |

### FEAT0105 - Mix Weighted Search

| Attribute          | Value                                                                              |
| ------------------ | ---------------------------------------------------------------------------------- |
| **ID**             | FEAT0105                                                                           |
| **Name**           | Mix Weighted Search                                                                |
| **Module**         | edgequake-query                                                                    |
| **Status**         | 📋 Planned                                                                         |
| **Code Reference** | [strategies.rs#MixStrategy](../edgequake/crates/edgequake-query/src/strategies.rs) |
| **Description**    | Configurable weight between naive and graph search                                 |
| **Related**        | FEAT0101, FEAT0102                                                                 |

### FEAT0106 - Bypass Mode

| Attribute          | Value                                                          |
| ------------------ | -------------------------------------------------------------- |
| **ID**             | FEAT0106                                                       |
| **Name**           | Bypass (No RAG) Mode                                           |
| **Module**         | edgequake-query                                                |
| **Status**         | ✅ Stable                                                      |
| **Code Reference** | [engine.rs](../edgequake/crates/edgequake-query/src/engine.rs) |
| **Description**    | Direct LLM query without RAG context                           |
| **Related**        | FEAT0007, UC0201                                               |

### FEAT0107 - Keyword Extraction

| Attribute          | Value                                                          |
| ------------------ | -------------------------------------------------------------- |
| **ID**             | FEAT0107                                                       |
| **Name**           | LLM-Based Keyword Extraction                                   |
| **Module**         | edgequake-query                                                |
| **Status**         | ✅ Stable                                                      |
| **Code Reference** | [keywords/](../edgequake/crates/edgequake-query/src/keywords/) |
| **Description**    | Extract search keywords from natural language queries          |
| **Related**        | FEAT0007, BR0101                                               |

### FEAT0108 - Context Truncation

| Attribute          | Value                                                                  |
| ------------------ | ---------------------------------------------------------------------- |
| **ID**             | FEAT0108                                                               |
| **Name**           | Smart Context Truncation                                               |
| **Module**         | edgequake-query                                                        |
| **Status**         | ✅ Stable                                                              |
| **Code Reference** | [truncation.rs](../edgequake/crates/edgequake-query/src/truncation.rs) |
| **Description**    | Truncate context to fit LLM token limits                               |
| **Related**        | BR0101, FEAT0007                                                       |

### FEAT0109 - SOTA Query Engine

| Attribute          | Value                                                                    |
| ------------------ | ------------------------------------------------------------------------ |
| **ID**             | FEAT0109                                                                 |
| **Name**           | State-of-the-Art Query Engine                                            |
| **Module**         | edgequake-query                                                          |
| **Status**         | ✅ Stable                                                                |
| **Code Reference** | [sota_engine.rs](../edgequake/crates/edgequake-query/src/sota_engine.rs) |
| **Description**    | Advanced query engine with optimized retrieval                           |
| **Related**        | FEAT0101-FEAT0108                                                        |

### FEAT0110 - Vector Filtering

| Attribute          | Value                                                                        |
| ------------------ | ---------------------------------------------------------------------------- |
| **ID**             | FEAT0110                                                                     |
| **Name**           | Vector Type Filtering                                                        |
| **Module**         | edgequake-query                                                              |
| **Status**         | ✅ Stable                                                                    |
| **Code Reference** | [vector_filter.rs](../edgequake/crates/edgequake-query/src/vector_filter.rs) |
| **Description**    | Filter vectors by type (chunk, entity, relationship)                         |
| **Related**        | FEAT0101, FEAT0109                                                           |

---

## Storage Features (FEAT02XX)

### FEAT0201 - In-Memory Storage

| Attribute          | Value                                                                          |
| ------------------ | ------------------------------------------------------------------------------ |
| **ID**             | FEAT0201                                                                       |
| **Name**           | In-Memory Storage Backend                                                      |
| **Module**         | edgequake-storage                                                              |
| **Status**         | ✅ Stable                                                                      |
| **Code Reference** | [adapters/memory/](../edgequake/crates/edgequake-storage/src/adapters/memory/) |
| **Description**    | Ephemeral storage for testing and development                                  |
| **Related**        | FEAT0202, BR0201                                                               |

### FEAT0202 - PostgreSQL KV Storage

| Attribute          | Value                                                                                        |
| ------------------ | -------------------------------------------------------------------------------------------- |
| **ID**             | FEAT0202                                                                                     |
| **Name**           | PostgreSQL Key-Value Storage                                                                 |
| **Module**         | edgequake-storage                                                                            |
| **Status**         | ✅ Stable                                                                                    |
| **Code Reference** | [adapters/postgres/kv.rs](../edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs) |
| **Description**    | Persistent KV storage using PostgreSQL JSONB                                                 |
| **Related**        | FEAT0201, BR0201                                                                             |

### FEAT0203 - pgvector Integration

| Attribute          | Value                                                                                                |
| ------------------ | ---------------------------------------------------------------------------------------------------- |
| **ID**             | FEAT0203                                                                                             |
| **Name**           | pgvector Vector Storage                                                                              |
| **Module**         | edgequake-storage                                                                                    |
| **Status**         | ✅ Stable                                                                                            |
| **Code Reference** | [adapters/postgres/vector.rs](../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs) |
| **Description**    | Vector similarity search using pgvector extension                                                    |
| **Related**        | FEAT0006, FEAT0101                                                                                   |

### FEAT0204 - Apache AGE Graph Storage

| Attribute          | Value                                                                                              |
| ------------------ | -------------------------------------------------------------------------------------------------- |
| **ID**             | FEAT0204                                                                                           |
| **Name**           | Apache AGE Graph Storage                                                                           |
| **Module**         | edgequake-storage                                                                                  |
| **Status**         | ✅ Stable                                                                                          |
| **Code Reference** | [adapters/postgres/graph.rs](../edgequake/crates/edgequake-storage/src/adapters/postgres/graph.rs) |
| **Description**    | Knowledge graph storage using Apache AGE extension                                                 |
| **Related**        | FEAT0005, FEAT0102                                                                                 |

### FEAT0205 - Community Detection

| Attribute          | Value                                                                  |
| ------------------ | ---------------------------------------------------------------------- |
| **ID**             | FEAT0205                                                               |
| **Name**           | Graph Community Detection                                              |
| **Module**         | edgequake-storage                                                      |
| **Status**         | ✅ Stable                                                              |
| **Code Reference** | [community.rs](../edgequake/crates/edgequake-storage/src/community.rs) |
| **Description**    | Detect communities in knowledge graph for global search                |
| **Related**        | FEAT0103, FEAT0204                                                     |

### FEAT0210 - Graph Storage Adapter

| Attribute          | Value                                                                          |
| ------------------ | ------------------------------------------------------------------------------ |
| **ID**             | FEAT0210                                                                       |
| **Name**           | Graph Storage Adapter                                                          |
| **Module**         | edgequake-storage                                                              |
| **Status**         | ✅ Stable                                                                      |
| **Code Reference** | [adapters/memory/mod.rs](../edgequake/crates/edgequake-storage/src/adapters/memory/mod.rs) |
| **Description**    | Graph storage interface for entity relationships                               |
| **Related**        | FEAT0201                                                                       |

### FEAT0211 - Vector Storage Adapter

| Attribute          | Value                                                                          |
| ------------------ | ------------------------------------------------------------------------------ |
| **ID**             | FEAT0211                                                                       |
| **Name**           | Vector Storage Adapter                                                         |
| **Module**         | edgequake-storage                                                              |
| **Status**         | ✅ Stable                                                                      |
| **Code Reference** | [adapters/memory/mod.rs](../edgequake/crates/edgequake-storage/src/adapters/memory/mod.rs) |
| **Description**    | Vector storage interface for similarity search                                 |
| **Related**        | FEAT0201                                                                       |

### FEAT0212 - KV Storage Adapter

| Attribute          | Value                                                                          |
| ------------------ | ------------------------------------------------------------------------------ |
| **ID**             | FEAT0212                                                                       |
| **Name**           | Key-Value Storage Adapter                                                      |
| **Module**         | edgequake-storage                                                              |
| **Status**         | ✅ Stable                                                                      |
| **Code Reference** | [adapters/memory/mod.rs](../edgequake/crates/edgequake-storage/src/adapters/memory/mod.rs) |
| **Description**    | Key-Value storage interface for document metadata                              |
| **Related**        | FEAT0201                                                                       |

---

## Pipeline Features (FEAT03XX)

### FEAT0301 - Character-Based Chunking

| Attribute          | Value                                                                                      |
| ------------------ | ------------------------------------------------------------------------------------------ |
| **ID**             | FEAT0301                                                                                   |
| **Name**           | Character-Based Chunking                                                                   |
| **Module**         | edgequake-pipeline                                                                         |
| **Status**         | ✅ Stable                                                                                  |
| **Code Reference** | [chunker.rs#CharacterBasedChunking](../edgequake/crates/edgequake-pipeline/src/chunker.rs) |
| **Description**    | Split text by character count with overlap                                                 |
| **Related**        | FEAT0002, BR0002                                                                           |

### FEAT0302 - Token-Based Chunking

| Attribute          | Value                                                                                  |
| ------------------ | -------------------------------------------------------------------------------------- |
| **ID**             | FEAT0302                                                                               |
| **Name**           | Token-Based Chunking                                                                   |
| **Module**         | edgequake-pipeline                                                                     |
| **Status**         | ✅ Stable                                                                              |
| **Code Reference** | [chunker.rs#TokenBasedChunking](../edgequake/crates/edgequake-pipeline/src/chunker.rs) |
| **Description**    | Split text by token count for LLM optimization                                         |
| **Related**        | FEAT0002, BR0002                                                                       |

### FEAT0303 - SOTA Tuple Extraction

| Attribute          | Value                                                                                 |
| ------------------ | ------------------------------------------------------------------------------------- |
| **ID**             | FEAT0303                                                                              |
| **Name**           | SOTA Tuple-Based Extraction                                                           |
| **Module**         | edgequake-pipeline                                                                    |
| **Status**         | ✅ Stable                                                                             |
| **Code Reference** | [extractor.rs#SOTAExtractor](../edgequake/crates/edgequake-pipeline/src/extractor.rs) |
| **Description**    | Extract entities using tuple format (more robust than JSON)                           |
| **Related**        | FEAT0003, BR0003                                                                      |

### FEAT0304 - Gleaning Extraction

| Attribute          | Value                                                                                     |
| ------------------ | ----------------------------------------------------------------------------------------- |
| **ID**             | FEAT0304                                                                                  |
| **Name**           | Multi-Pass Gleaning Extraction                                                            |
| **Module**         | edgequake-pipeline                                                                        |
| **Status**         | ✅ Stable                                                                                 |
| **Code Reference** | [extractor.rs#GleaningExtractor](../edgequake/crates/edgequake-pipeline/src/extractor.rs) |
| **Description**    | Multiple extraction passes to improve recall                                              |
| **Related**        | FEAT0003, FEAT0303                                                                        |

---

## API Features (FEAT04XX)

### FEAT0401 - Document Upload (Text)

| Attribute          | Value                                                                                |
| ------------------ | ------------------------------------------------------------------------------------ |
| **ID**             | FEAT0401                                                                             |
| **Name**           | Document Upload via Text                                                             |
| **Module**         | edgequake-api                                                                        |
| **Status**         | ✅ Stable                                                                            |
| **Code Reference** | [handlers/documents.rs](../edgequake/crates/edgequake-api/src/handlers/documents.rs) |
| **Description**    | Upload document content as JSON text                                                 |
| **Related**        | UC0001, FEAT0001                                                                     |

### FEAT0402 - Document Upload (File)

| Attribute          | Value                                                                                |
| ------------------ | ------------------------------------------------------------------------------------ |
| **ID**             | FEAT0402                                                                             |
| **Name**           | Document Upload via File                                                             |
| **Module**         | edgequake-api                                                                        |
| **Status**         | ✅ Stable                                                                            |
| **Code Reference** | [handlers/documents.rs](../edgequake/crates/edgequake-api/src/handlers/documents.rs) |
| **Description**    | Upload PDF, TXT, MD files via multipart form                                         |
| **Related**        | UC0002, FEAT0001, FEAT0501                                                           |

### FEAT0403 - Query Execution Endpoint

| Attribute          | Value                                                                        |
| ------------------ | ---------------------------------------------------------------------------- |
| **ID**             | FEAT0403                                                                     |
| **Name**           | Query Execution API                                                          |
| **Module**         | edgequake-api                                                                |
| **Status**         | ✅ Stable                                                                    |
| **Code Reference** | [handlers/query.rs](../edgequake/crates/edgequake-api/src/handlers/query.rs) |
| **Description**    | Execute RAG queries via POST endpoint                                        |
| **Related**        | UC0201, FEAT0007                                                             |

### FEAT0404 - Query Streaming Endpoint

| Attribute          | Value                                                                        |
| ------------------ | ---------------------------------------------------------------------------- |
| **ID**             | FEAT0404                                                                     |
| **Name**           | Query Streaming API                                                          |
| **Module**         | edgequake-api                                                                |
| **Status**         | ✅ Stable                                                                    |
| **Code Reference** | [handlers/query.rs](../edgequake/crates/edgequake-api/src/handlers/query.rs) |
| **Description**    | Stream query responses via SSE                                               |
| **Related**        | UC0202, FEAT0008                                                             |

### FEAT0405 - Graph Exploration API

| Attribute          | Value                                                                        |
| ------------------ | ---------------------------------------------------------------------------- |
| **ID**             | FEAT0405                                                                     |
| **Name**           | Graph Exploration API                                                        |
| **Module**         | edgequake-api                                                                |
| **Status**         | 📋 Planned                                                                   |
| **Code Reference** | [handlers/graph.rs](../edgequake/crates/edgequake-api/src/handlers/graph.rs) |
| **Description**    | Browse knowledge graph entities and relationships                            |
| **Related**        | UC0101-UC0105, FEAT0005                                                      |

### FEAT0406 - Task Status API

| Attribute          | Value                                                                        |
| ------------------ | ---------------------------------------------------------------------------- |
| **ID**             | FEAT0406                                                                     |
| **Name**           | Task Status Tracking API                                                     |
| **Module**         | edgequake-api                                                                |
| **Status**         | ✅ Stable                                                                    |
| **Code Reference** | [handlers/tasks.rs](../edgequake/crates/edgequake-api/src/handlers/tasks.rs) |
| **Description**    | Get status of background processing tasks                                    |
| **Related**        | UC0005, FEAT0019                                                             |

---

## PDF Features (FEAT05XX)

### FEAT0501 - PDF Text Extraction

| Attribute          | Value                                                                                              |
| ------------------ | -------------------------------------------------------------------------------------------------- |
| **ID**             | FEAT0501                                                                                           |
| **Name**           | PDF Text Extraction                                                                                |
| **Module**         | edgequake-pdf                                                                                      |
| **Status**         | ✅ Stable                                                                                          |
| **Code Reference** | [backend/extraction_engine.rs](../edgequake/crates/edgequake-pdf/src/backend/extraction_engine.rs) |
| **Description**    | Extract text content from PDF documents                                                            |
| **Related**        | FEAT0402, UC0002                                                                                   |

### FEAT0502 - PDF Layout Analysis

| Attribute          | Value                                                    |
| ------------------ | -------------------------------------------------------- |
| **ID**             | FEAT0502                                                 |
| **Name**           | PDF Layout Analysis                                      |
| **Module**         | edgequake-pdf                                            |
| **Status**         | ✅ Stable                                                |
| **Code Reference** | [layout/](../edgequake/crates/edgequake-pdf/src/layout/) |
| **Description**    | Analyze PDF page layout for structure detection          |
| **Related**        | FEAT0501, FEAT0503                                       |

### FEAT0503 - Table Detection

| Attribute          | Value                                                                          |
| ------------------ | ------------------------------------------------------------------------------ |
| **ID**             | FEAT0503                                                                       |
| **Name**           | PDF Table Detection                                                            |
| **Module**         | edgequake-pdf                                                                  |
| **Status**         | ✅ Stable                                                                      |
| **Code Reference** | [backend/lattice.rs](../edgequake/crates/edgequake-pdf/src/backend/lattice.rs) |
| **Description**    | Detect and extract tables from PDF documents                                   |
| **Related**        | FEAT0501, FEAT0504                                                             |

### FEAT0504 - Markdown Rendering

| Attribute          | Value                                                          |
| ------------------ | -------------------------------------------------------------- |
| **ID**             | FEAT0504                                                       |
| **Name**           | PDF to Markdown Rendering                                      |
| **Module**         | edgequake-pdf                                                  |
| **Status**         | ✅ Stable                                                      |
| **Code Reference** | [renderers/](../edgequake/crates/edgequake-pdf/src/renderers/) |
| **Description**    | Convert PDF content to Markdown format                         |
| **Related**        | FEAT0501, FEAT0503                                             |

### FEAT0505 - Heading Detection

| Attribute          | Value                                                                                                        |
| ------------------ | ------------------------------------------------------------------------------------------------------------ |
| **ID**             | FEAT0505                                                                                                     |
| **Name**           | PDF Heading Detection                                                                                        |
| **Module**         | edgequake-pdf                                                                                                |
| **Status**         | ✅ Stable                                                                                                    |
| **Code Reference** | [processors/structure_detection.rs](../edgequake/crates/edgequake-pdf/src/processors/structure_detection.rs) |
| **Description**    | Detect headings based on font size and formatting                                                            |
| **Related**        | FEAT0502, FEAT0504                                                                                           |

---

## WebUI Features (FEAT06XX)

> The EdgeQuake WebUI is a Next.js 16 + React 19 application with Sigma.js for graph visualization.
> State is managed via Zustand stores and TanStack Query for server state.

### FEAT0601 - Knowledge Graph Visualization

| Attribute          | Value                                                                                     |
| ------------------ | ----------------------------------------------------------------------------------------- |
| **ID**             | FEAT0601                                                                                  |
| **Name**           | Knowledge Graph Visualization                                                             |
| **Module**         | edgequake_webui                                                                           |
| **Status**         | ✅ Stable                                                                                 |
| **Code Reference** | [stores/use-graph-store.ts](../edgequake_webui/src/stores/use-graph-store.ts) (950 lines) |
| **Description**    | Interactive WebGL graph rendering with Sigma.js, node selection, zoom/pan, filtering      |
| **Related**        | FEAT0607, FEAT0608, FEAT0616, UC0101                                                      |

### FEAT0602 - Chat Query Interface

| Attribute          | Value                                                                         |
| ------------------ | ----------------------------------------------------------------------------- |
| **ID**             | FEAT0602                                                                      |
| **Name**           | Chat Query Interface                                                          |
| **Module**         | edgequake_webui                                                               |
| **Status**         | ✅ Stable                                                                     |
| **Code Reference** | [stores/use-query-store.ts](../edgequake_webui/src/stores/use-query-store.ts) |
| **Description**    | Conversational RAG interface with message history and context display         |
| **Related**        | FEAT0603, FEAT0609, UC0201                                                    |

### FEAT0603 - Streaming Response Display

| Attribute          | Value                                                                         |
| ------------------ | ----------------------------------------------------------------------------- |
| **ID**             | FEAT0603                                                                      |
| **Name**           | Streaming Response Display                                                    |
| **Module**         | edgequake_webui                                                               |
| **Status**         | ✅ Stable                                                                     |
| **Code Reference** | [hooks/use-graph-stream.ts](../edgequake_webui/src/hooks/use-graph-stream.ts) |
| **Description**    | SSE streaming with progressive markdown rendering and typing animation        |
| **Related**        | FEAT0602, FEAT0008, BR0007                                                    |

### FEAT0604 - Query Mode Selector

| Attribute          | Value                                                                               |
| ------------------ | ----------------------------------------------------------------------------------- |
| **ID**             | FEAT0604                                                                            |
| **Name**           | Query Mode Selector                                                                 |
| **Module**         | edgequake_webui                                                                     |
| **Status**         | ✅ Stable                                                                           |
| **Code Reference** | [stores/use-query-ui-store.ts](../edgequake_webui/src/stores/use-query-ui-store.ts) |
| **Description**    | Select query mode: hybrid, local, global, naive, mix, bypass                        |
| **Related**        | FEAT0007, FEAT0101-FEAT0106                                                         |

### FEAT0605 - Document Upload Interface

| Attribute          | Value                                                                                 |
| ------------------ | ------------------------------------------------------------------------------------- |
| **ID**             | FEAT0605                                                                              |
| **Name**           | Document Upload Interface                                                             |
| **Module**         | edgequake_webui                                                                       |
| **Status**         | ✅ Stable                                                                             |
| **Code Reference** | [stores/use-ingestion-store.ts](../edgequake_webui/src/stores/use-ingestion-store.ts) |
| **Description**    | Drag-and-drop file upload with progress tracking and batch support                    |
| **Related**        | FEAT0611, FEAT0001, UC0001, UC0002                                                    |

### FEAT0606 - Workspace Switcher

| Attribute          | Value                                                                           |
| ------------------ | ------------------------------------------------------------------------------- |
| **ID**             | FEAT0606                                                                        |
| **Name**           | Workspace Switcher                                                              |
| **Module**         | edgequake_webui                                                                 |
| **Status**         | ✅ Stable                                                                       |
| **Code Reference** | [stores/use-tenant-store.ts](../edgequake_webui/src/stores/use-tenant-store.ts) |
| **Description**    | Switch between workspaces with tenant isolation                                 |
| **Related**        | FEAT0016, FEAT0015, UC0301                                                      |

### FEAT0607 - Entity Type Filter

| Attribute          | Value                                                                         |
| ------------------ | ----------------------------------------------------------------------------- |
| **ID**             | FEAT0607                                                                      |
| **Name**           | Entity Type Filter                                                            |
| **Module**         | edgequake_webui                                                               |
| **Status**         | ✅ Stable                                                                     |
| **Code Reference** | [stores/use-graph-store.ts](../edgequake_webui/src/stores/use-graph-store.ts) |
| **Description**    | Filter graph nodes by entity type (PERSON, ORG, CONCEPT, etc.)                |
| **Related**        | FEAT0601, UC0101                                                              |

### FEAT0608 - Graph Bookmark Manager

| Attribute          | Value                                                                                       |
| ------------------ | ------------------------------------------------------------------------------------------- |
| **ID**             | FEAT0608                                                                                    |
| **Name**           | Graph Bookmark Manager                                                                      |
| **Module**         | edgequake_webui                                                                             |
| **Status**         | ✅ Stable                                                                                   |
| **Code Reference** | [stores/use-graph-store.ts#GraphBookmark](../edgequake_webui/src/stores/use-graph-store.ts) |
| **Description**    | Save and restore graph view states (camera, visible nodes, filters)                         |
| **Related**        | FEAT0601, FEAT0617                                                                          |

### FEAT0609 - Conversation Persistence

| Attribute          | Value                                                                                       |
| ------------------ | ------------------------------------------------------------------------------------------- |
| **ID**             | FEAT0609                                                                                    |
| **Name**           | Conversation Persistence                                                                    |
| **Module**         | edgequake_webui                                                                             |
| **Status**         | ✅ Stable                                                                                   |
| **Code Reference** | [stores/use-conversation-store.ts](../edgequake_webui/src/stores/use-conversation-store.ts) |
| **Description**    | Persist conversation history to localStorage and sync with backend                          |
| **Related**        | FEAT0602, FEAT0017, UC0401                                                                  |

### FEAT0610 - Cost Tracking Display

| Attribute          | Value                                                                       |
| ------------------ | --------------------------------------------------------------------------- |
| **ID**             | FEAT0610                                                                    |
| **Name**           | Cost Tracking Display                                                       |
| **Module**         | edgequake_webui                                                             |
| **Status**         | ✅ Stable                                                                   |
| **Code Reference** | [stores/use-cost-store.ts](../edgequake_webui/src/stores/use-cost-store.ts) |
| **Description**    | Display token usage and estimated costs for LLM operations                  |
| **Related**        | FEAT0013, BR0301                                                            |

### FEAT0611 - Ingestion Progress Monitor

| Attribute          | Value                                                                                     |
| ------------------ | ----------------------------------------------------------------------------------------- |
| **ID**             | FEAT0611                                                                                  |
| **Name**           | Ingestion Progress Monitor                                                                |
| **Module**         | edgequake_webui                                                                           |
| **Status**         | ✅ Stable                                                                                 |
| **Code Reference** | [hooks/use-ingestion-progress.ts](../edgequake_webui/src/hooks/use-ingestion-progress.ts) |
| **Description**    | Real-time document processing progress via SSE                                            |
| **Related**        | FEAT0605, FEAT0012, UC0001                                                                |

### FEAT0612 - Keyboard Navigation

| Attribute          | Value                                                                                     |
| ------------------ | ----------------------------------------------------------------------------------------- |
| **ID**             | FEAT0612                                                                                  |
| **Name**           | Keyboard Navigation                                                                       |
| **Module**         | edgequake_webui                                                                           |
| **Status**         | ✅ Stable                                                                                 |
| **Code Reference** | [hooks/use-keyboard-shortcuts.ts](../edgequake_webui/src/hooks/use-keyboard-shortcuts.ts) |
| **Description**    | Keyboard shortcuts for graph navigation (arrow keys, +/-, ESC)                            |
| **Related**        | FEAT0601, FEAT0616                                                                        |

### FEAT0613 - Dark/Light Theme

| Attribute          | Value                                                                               |
| ------------------ | ----------------------------------------------------------------------------------- |
| **ID**             | FEAT0613                                                                            |
| **Name**           | Dark/Light Theme Toggle                                                             |
| **Module**         | edgequake_webui                                                                     |
| **Status**         | ✅ Stable                                                                           |
| **Code Reference** | [stores/use-settings-store.ts](../edgequake_webui/src/stores/use-settings-store.ts) |
| **Description**    | Toggle between dark and light themes with system preference detection               |
| **Related**        | FEAT0617, BR0609                                                                    |

### FEAT0614 - Multi-Language (i18n)

| Attribute          | Value                                                    |
| ------------------ | -------------------------------------------------------- |
| **ID**             | FEAT0614                                                 |
| **Name**           | Multi-Language Support                                   |
| **Module**         | edgequake_webui                                          |
| **Status**         | ✅ Stable                                                |
| **Code Reference** | [locales/](../edgequake_webui/src/locales/)              |
| **Description**    | Internationalization via i18next with language detection |
| **Related**        | FEAT0617                                                 |

### FEAT0615 - Source Citation Links

| Attribute          | Value                                                               |
| ------------------ | ------------------------------------------------------------------- |
| **ID**             | FEAT0615                                                            |
| **Name**           | Source Citation Links                                               |
| **Module**         | edgequake_webui                                                     |
| **Status**         | ✅ Stable                                                           |
| **Code Reference** | [hooks/use-lineage.ts](../edgequake_webui/src/hooks/use-lineage.ts) |
| **Description**    | Deep links to source documents with line number references          |
| **Related**        | FEAT0011, FEAT0602, UC0202                                          |

### FEAT0616 - Entity Search (MiniSearch)

| Attribute          | Value                                                                         |
| ------------------ | ----------------------------------------------------------------------------- |
| **ID**             | FEAT0616                                                                      |
| **Name**           | Entity Search                                                                 |
| **Module**         | edgequake_webui                                                               |
| **Status**         | ✅ Stable                                                                     |
| **Code Reference** | [stores/use-graph-store.ts](../edgequake_webui/src/stores/use-graph-store.ts) |
| **Description**    | Client-side fuzzy search using MiniSearch library                             |
| **Related**        | FEAT0601, FEAT0607                                                            |

### FEAT0617 - User Preference Persistence

| Attribute          | Value                                                                               |
| ------------------ | ----------------------------------------------------------------------------------- |
| **ID**             | FEAT0617                                                                            |
| **Name**           | User Preference Persistence                                                         |
| **Module**         | edgequake_webui                                                                     |
| **Status**         | ✅ Stable                                                                           |
| **Code Reference** | [stores/use-settings-store.ts](../edgequake_webui/src/stores/use-settings-store.ts) |
| **Description**    | Persist user preferences to localStorage with cross-tab sync                        |
| **Related**        | FEAT0613, FEAT0618, FEAT0619, BR0609                                                |

### FEAT0618 - Graph Layout Settings

| Attribute          | Value                                                                               |
| ------------------ | ----------------------------------------------------------------------------------- |
| **ID**             | FEAT0618                                                                            |
| **Name**           | Graph Layout Settings                                                               |
| **Module**         | edgequake_webui                                                                     |
| **Status**         | ✅ Stable                                                                           |
| **Code Reference** | [stores/use-settings-store.ts](../edgequake_webui/src/stores/use-settings-store.ts) |
| **Description**    | Configure graph layout algorithm (Force, ForceAtlas2, Circular, Random)             |
| **Related**        | FEAT0601, FEAT0617                                                                  |

### FEAT0619 - Ingestion Quality Settings

| Attribute          | Value                                                                               |
| ------------------ | ----------------------------------------------------------------------------------- |
| **ID**             | FEAT0619                                                                            |
| **Name**           | Ingestion Quality Settings                                                          |
| **Module**         | edgequake_webui                                                                     |
| **Status**         | ✅ Stable                                                                           |
| **Code Reference** | [stores/use-settings-store.ts](../edgequake_webui/src/stores/use-settings-store.ts) |
| **Description**    | Configure gleaning iterations, summarization, and chunk settings                    |
| **Related**        | FEAT0605, FEAT0617, FEAT0002                                                        |

### FEAT0620 - Query Result Export

| Attribute          | Value                                                  |
| ------------------ | ------------------------------------------------------ |
| **ID**             | FEAT0620                                               |
| **Name**           | Query Result Export                                    |
| **Module**         | edgequake_webui                                        |
| **Status**         | 🔧 Planned                                             |
| **Code Reference** | -                                                      |
| **Description**    | Export query results to JSON, CSV, or Markdown formats |
| **Related**        | FEAT0602, UC0205                                       |

---

## WebUI API Client Features (FEAT07XX)

> TypeScript API client library for EdgeQuake backend integration.
> All features in `edgequake_webui/src/lib/api/` and `src/lib/`.

### FEAT0700 - Unified API Client

| Attribute          | Value                                                         |
| ------------------ | ------------------------------------------------------------- |
| **ID**             | FEAT0700                                                      |
| **Name**           | Unified API Client                                            |
| **Module**         | edgequake_webui                                               |
| **Status**         | ✅ Stable                                                     |
| **Code Reference** | [lib/api/client.ts](../edgequake_webui/src/lib/api/client.ts) |
| **Description**    | Centralized HTTP client with error handling and retries       |
| **Related**        | FEAT0701, FEAT0702, BR0607                                    |

### FEAT0701 - SSE Streaming Client

| Attribute          | Value                                                         |
| ------------------ | ------------------------------------------------------------- |
| **ID**             | FEAT0701                                                      |
| **Name**           | Server-Sent Events Streaming Client                           |
| **Module**         | edgequake_webui                                               |
| **Status**         | ✅ Stable                                                     |
| **Code Reference** | [lib/api/client.ts](../edgequake_webui/src/lib/api/client.ts) |
| **Description**    | SSE connection management for real-time updates               |
| **Related**        | FEAT0609, FEAT0611, BR0604                                    |

### FEAT0702 - Request/Response Interceptors

| Attribute          | Value                                                         |
| ------------------ | ------------------------------------------------------------- |
| **ID**             | FEAT0702                                                      |
| **Name**           | HTTP Interceptors                                             |
| **Module**         | edgequake_webui                                               |
| **Status**         | ✅ Stable                                                     |
| **Code Reference** | [lib/api/client.ts](../edgequake_webui/src/lib/api/client.ts) |
| **Description**    | Global request/response transformation and logging            |
| **Related**        | FEAT0700, BR0607                                              |

### FEAT0703 - Chat Completions API

| Attribute          | Value                                                     |
| ------------------ | --------------------------------------------------------- |
| **ID**             | FEAT0703                                                  |
| **Name**           | Chat Completions API Client                               |
| **Module**         | edgequake_webui                                           |
| **Status**         | ✅ Stable                                                 |
| **Code Reference** | [lib/api/chat.ts](../edgequake_webui/src/lib/api/chat.ts) |
| **Description**    | OpenAI-compatible chat API integration                    |
| **Related**        | FEAT0704, FEAT0705, UC0602                                |

### FEAT0704 - Streaming Chat Responses

| Attribute          | Value                                                     |
| ------------------ | --------------------------------------------------------- |
| **ID**             | FEAT0704                                                  |
| **Name**           | Streaming Chat Responses                                  |
| **Module**         | edgequake_webui                                           |
| **Status**         | ✅ Stable                                                 |
| **Code Reference** | [lib/api/chat.ts](../edgequake_webui/src/lib/api/chat.ts) |
| **Description**    | Real-time token-by-token response rendering               |
| **Related**        | FEAT0703, FEAT0701, BR0604, BR0612                        |

### FEAT0705 - Query Mode Selection

| Attribute          | Value                                                     |
| ------------------ | --------------------------------------------------------- |
| **ID**             | FEAT0705                                                  |
| **Name**           | Query Mode Selection                                      |
| **Module**         | edgequake_webui                                           |
| **Status**         | ✅ Stable                                                 |
| **Code Reference** | [lib/api/chat.ts](../edgequake_webui/src/lib/api/chat.ts) |
| **Description**    | Select RAG mode: naive/local/global/hybrid/mix/bypass     |
| **Related**        | FEAT0007, FEAT0101-0106                                   |

### FEAT0706 - Conversation List Pagination

| Attribute          | Value                                                                       |
| ------------------ | --------------------------------------------------------------------------- |
| **ID**             | FEAT0706                                                                    |
| **Name**           | Conversation List with Pagination                                           |
| **Module**         | edgequake_webui                                                             |
| **Status**         | ✅ Stable                                                                   |
| **Code Reference** | [lib/api/conversations.ts](../edgequake_webui/src/lib/api/conversations.ts) |
| **Description**    | Fetch conversation list with cursor-based pagination                        |
| **Related**        | FEAT0610, UC0604                                                            |

### FEAT0707 - Message History Retrieval

| Attribute          | Value                                                                       |
| ------------------ | --------------------------------------------------------------------------- |
| **ID**             | FEAT0707                                                                    |
| **Name**           | Message History Retrieval                                                   |
| **Module**         | edgequake_webui                                                             |
| **Status**         | ✅ Stable                                                                   |
| **Code Reference** | [lib/api/conversations.ts](../edgequake_webui/src/lib/api/conversations.ts) |
| **Description**    | Load full conversation history from backend                                 |
| **Related**        | FEAT0706, BR0602                                                            |

### FEAT0708 - Conversation Sharing

| Attribute          | Value                                                                       |
| ------------------ | --------------------------------------------------------------------------- |
| **ID**             | FEAT0708                                                                    |
| **Name**           | Conversation Sharing                                                        |
| **Module**         | edgequake_webui                                                             |
| **Status**         | ✅ Stable                                                                   |
| **Code Reference** | [lib/api/conversations.ts](../edgequake_webui/src/lib/api/conversations.ts) |
| **Description**    | Generate shareable links for conversations                                  |
| **Related**        | FEAT0706                                                                    |

### FEAT0709 - Folder CRUD Operations

| Attribute          | Value                                                           |
| ------------------ | --------------------------------------------------------------- |
| **ID**             | FEAT0709                                                        |
| **Name**           | Folder CRUD Operations                                          |
| **Module**         | edgequake_webui                                                 |
| **Status**         | ✅ Stable                                                       |
| **Code Reference** | [lib/api/folders.ts](../edgequake_webui/src/lib/api/folders.ts) |
| **Description**    | Create, read, update, delete conversation folders               |
| **Related**        | FEAT0710                                                        |

### FEAT0710 - Move Conversations to Folders

| Attribute          | Value                                                           |
| ------------------ | --------------------------------------------------------------- |
| **ID**             | FEAT0710                                                        |
| **Name**           | Move Conversations to Folders                                   |
| **Module**         | edgequake_webui                                                 |
| **Status**         | ✅ Stable                                                       |
| **Code Reference** | [lib/api/folders.ts](../edgequake_webui/src/lib/api/folders.ts) |
| **Description**    | Organize conversations into custom folder hierarchy             |
| **Related**        | FEAT0709, FEAT0706                                              |

### FEAT0711 - Hierarchical Query Keys

| Attribute          | Value                                                                 |
| ------------------ | --------------------------------------------------------------------- |
| **ID**             | FEAT0711                                                              |
| **Name**           | Hierarchical Query Keys                                               |
| **Module**         | edgequake_webui                                                       |
| **Status**         | ✅ Stable                                                             |
| **Code Reference** | [lib/api/query-keys.ts](../edgequake_webui/src/lib/api/query-keys.ts) |
| **Description**    | TanStack Query key factory for consistent cache keying                |
| **Related**        | FEAT0712                                                              |

### FEAT0712 - Automatic Cache Invalidation

| Attribute          | Value                                                                 |
| ------------------ | --------------------------------------------------------------------- |
| **ID**             | FEAT0712                                                              |
| **Name**           | Automatic Cache Invalidation                                          |
| **Module**         | edgequake_webui                                                       |
| **Status**         | ✅ Stable                                                             |
| **Code Reference** | [lib/api/query-keys.ts](../edgequake_webui/src/lib/api/query-keys.ts) |
| **Description**    | Invalidate dependent queries on mutations                             |
| **Related**        | FEAT0711                                                              |

### FEAT0713 - Camera Focus on Node

| Attribute          | Value                                                                         |
| ------------------ | ----------------------------------------------------------------------------- |
| **ID**             | FEAT0713                                                                      |
| **Name**           | Camera Focus on Graph Node                                                    |
| **Module**         | edgequake_webui                                                               |
| **Status**         | ✅ Stable                                                                     |
| **Code Reference** | [lib/graph/camera-utils.ts](../edgequake_webui/src/lib/graph/camera-utils.ts) |
| **Description**    | Smooth camera animation to focus on selected node                             |
| **Related**        | FEAT0601, UC0609                                                              |

### FEAT0727 - Export Conversation to Markdown

| Attribute          | Value                                                                           |
| ------------------ | ------------------------------------------------------------------------------- |
| **ID**             | FEAT0727                                                                        |
| **Name**           | Export Conversation to Markdown                                                 |
| **Module**         | edgequake_webui                                                                 |
| **Status**         | ✅ Stable                                                                       |
| **Code Reference** | [lib/export-conversation.ts](../edgequake_webui/src/lib/export-conversation.ts) |
| **Description**    | Download conversation as formatted Markdown file                                |
| **Related**        | FEAT0728, FEAT0706                                                              |

### FEAT0728 - Export Conversation to JSON

| Attribute          | Value                                                                           |
| ------------------ | ------------------------------------------------------------------------------- |
| **ID**             | FEAT0728                                                                        |
| **Name**           | Export Conversation to JSON                                                     |
| **Module**         | edgequake_webui                                                                 |
| **Status**         | ✅ Stable                                                                       |
| **Code Reference** | [lib/export-conversation.ts](../edgequake_webui/src/lib/export-conversation.ts) |
| **Description**    | Download conversation as structured JSON file                                   |
| **Related**        | FEAT0727, FEAT0706                                                              |

### FEAT0733 - Tailwind Class Merging

| Attribute          | Value                                                                        |
| ------------------ | ---------------------------------------------------------------------------- |
| **ID**             | FEAT0733                                                                     |
| **Name**           | Tailwind CSS Class Merging                                                   |
| **Module**         | edgequake_webui                                                              |
| **Status**         | ✅ Stable                                                                    |
| **Code Reference** | [lib/utils.ts](../edgequake_webui/src/lib/utils.ts)                          |
| **Description**    | Merge conflicting Tailwind classes intelligently (via clsx + tailwind-merge) |
| **Related**        | FEAT0619                                                                     |

### FEAT0734 - Chain-of-Thought Display

| Attribute          | Value                                                                                |
| ------------------ | ------------------------------------------------------------------------------------ |
| **ID**             | FEAT0734                                                                             |
| **Name**           | Chain-of-Thought Display                                                             |
| **Module**         | edgequake_webui                                                                      |
| **Status**         | ✅ Stable                                                                            |
| **Code Reference** | [thinking-display.tsx](../edgequake_webui/src/components/query/thinking-display.tsx) |
| **Description**    | Display LLM reasoning steps with expandable thinking sections in query interface     |
| **Related**        | FEAT0770, FEAT0771                                                                   |

---

## Auth Features (FEAT08XX)

### FEAT0801 - API Key Authentication

| Attribute          | Value                                                                 |
| ------------------ | --------------------------------------------------------------------- |
| **ID**             | FEAT0801                                                              |
| **Name**           | API Key Authentication                                                |
| **Module**         | edgequake-auth                                                        |
| **Status**         | ✅ Stable                                                             |
| **Code Reference** | [extractors.rs](../edgequake/crates/edgequake-auth/src/extractors.rs) |
| **Description**    | Authenticate requests via API key header                              |
| **Related**        | BR0201, FEAT0015                                                      |

### FEAT0802 - JWT Token Support

| Attribute          | Value                                                   |
| ------------------ | ------------------------------------------------------- |
| **ID**             | FEAT0802                                                |
| **Name**           | JWT Token Authentication                                |
| **Module**         | edgequake-auth                                          |
| **Status**         | ✅ Stable                                               |
| **Code Reference** | [jwt.rs](../edgequake/crates/edgequake-auth/src/jwt.rs) |
| **Description**    | Support JWT tokens for web client authentication        |
| **Related**        | FEAT0801, BR0201                                        |

### FEAT0803 - Role-Based Access Control

| Attribute          | Value                                                     |
| ------------------ | --------------------------------------------------------- |
| **ID**             | FEAT0803                                                  |
| **Name**           | RBAC Authorization                                        |
| **Module**         | edgequake-auth                                            |
| **Status**         | ✅ Stable                                                 |
| **Code Reference** | [rbac.rs](../edgequake/crates/edgequake-auth/src/rbac.rs) |
| **Description**    | Role-based permission checks                              |
| **Related**        | BR0202, FEAT0801                                          |

### FEAT0804 - JWT Login

| Attribute          | Value                                                              |
| ------------------ | ------------------------------------------------------------------ |
| **ID**             | FEAT0804                                                           |
| **Name**           | JWT Login and Tokens                                               |
| **Module**         | edgequake-api                                                      |
| **Status**         | ✅ Stable                                                          |
| **Code Reference** | [handlers/auth.rs](../edgequake/crates/edgequake-api/src/handlers/auth.rs) |
| **Description**    | JWT login with access and refresh tokens                           |
| **Related**        | FEAT0802                                                           |

### FEAT0805 - Token Refresh

| Attribute          | Value                                                              |
| ------------------ | ------------------------------------------------------------------ |
| **ID**             | FEAT0805                                                           |
| **Name**           | Token Refresh                                                      |
| **Module**         | edgequake-api                                                      |
| **Status**         | ✅ Stable                                                          |
| **Code Reference** | [handlers/auth.rs](../edgequake/crates/edgequake-api/src/handlers/auth.rs) |
| **Description**    | Token refresh without re-authentication                            |
| **Related**        | FEAT0804                                                           |

### FEAT0806 - User Management

| Attribute          | Value                                                              |
| ------------------ | ------------------------------------------------------------------ |
| **ID**             | FEAT0806                                                           |
| **Name**           | User Management (CRUD)                                             |
| **Module**         | edgequake-api                                                      |
| **Status**         | ✅ Stable                                                          |
| **Code Reference** | [handlers/auth.rs](../edgequake/crates/edgequake-api/src/handlers/auth.rs) |
| **Description**    | User CRUD operations with role management                          |
| **Related**        | FEAT0803                                                           |

### FEAT0807 - API Key Management

| Attribute          | Value                                                              |
| ------------------ | ------------------------------------------------------------------ |
| **ID**             | FEAT0807                                                           |
| **Name**           | API Key Management                                                 |
| **Module**         | edgequake-api                                                      |
| **Status**         | ✅ Stable                                                          |
| **Code Reference** | [handlers/auth.rs](../edgequake/crates/edgequake-api/src/handlers/auth.rs) |
| **Description**    | API key generation and validation                                  |
| **Related**        | FEAT0801                                                           |

### FEAT0820 - Workspace Management

| Attribute          | Value                                                              |
| ------------------ | ------------------------------------------------------------------ |
| **ID**             | FEAT0820                                                           |
| **Name**           | Workspace CRUD                                                     |
| **Module**         | edgequake-core                                                     |
| **Status**         | ✅ Stable                                                          |
| **Code Reference** | [workspace_service.rs](../edgequake/crates/edgequake-core/src/workspace_service.rs) |
| **Description**    | Workspace CRUD operations                                          |
| **Related**        | FEAT0016                                                           |

### FEAT0821 - Tenant Management

| Attribute          | Value                                                              |
| ------------------ | ------------------------------------------------------------------ |
| **ID**             | FEAT0821                                                           |
| **Name**           | Tenant Management                                                  |
| **Module**         | edgequake-core                                                     |
| **Status**         | ✅ Stable                                                          |
| **Code Reference** | [workspace_service.rs](../edgequake/crates/edgequake-core/src/workspace_service.rs) |
| **Description**    | Tenant configuration and management                                |
| **Related**        | FEAT0015                                                           |

### FEAT0822 - Membership Management

| Attribute          | Value                                                              |
| ------------------ | ------------------------------------------------------------------ |
| **ID**             | FEAT0822                                                           |
| **Name**           | Membership Management                                              |
| **Module**         | edgequake-core                                                     |
| **Status**         | ✅ Stable                                                          |
| **Code Reference** | [workspace_service.rs](../edgequake/crates/edgequake-core/src/workspace_service.rs) |
| **Description**    | Membership and role management                                     |
| **Related**        | FEAT0820                                                           |

### FEAT0823 - Workspace Statistics

| Attribute          | Value                                                              |
| ------------------ | ------------------------------------------------------------------ |
| **ID**             | FEAT0823                                                           |
| **Name**           | Workspace Statistics                                               |
| **Module**         | edgequake-core                                                     |
| **Status**         | ✅ Stable                                                          |
| **Code Reference** | [workspace_service.rs](../edgequake/crates/edgequake-core/src/workspace_service.rs) |
| **Description**    | Workspace usage statistics                                         |
| **Related**        | FEAT0820                                                           |

### FEAT0830 - Tenant Instance Management

| Attribute          | Value                                                              |
| ------------------ | ------------------------------------------------------------------ |
| **ID**             | FEAT0830                                                           |
| **Name**           | Tenant Instance Management                                         |
| **Module**         | edgequake-core                                                     |
| **Status**         | ✅ Stable                                                          |
| **Code Reference** | [tenant_manager.rs](../edgequake/crates/edgequake-core/src/tenant_manager.rs) |
| **Description**    | Per-tenant EdgeQuake instance management                           |
| **Related**        | FEAT0015                                                           |

### FEAT0831 - Instance Caching

| Attribute          | Value                                                              |
| ------------------ | ------------------------------------------------------------------ |
| **ID**             | FEAT0831                                                           |
| **Name**           | Instance Caching                                                   |
| **Module**         | edgequake-core                                                     |
| **Status**         | ✅ Stable                                                          |
| **Code Reference** | [tenant_manager.rs](../edgequake/crates/edgequake-core/src/tenant_manager.rs) |
| **Description**    | Instance caching for performance                                   |
| **Related**        | FEAT0830                                                           |

### FEAT0832 - Instance Cleanup

| Attribute          | Value                                                              |
| ------------------ | ------------------------------------------------------------------ |
| **ID**             | FEAT0832                                                           |
| **Name**           | Stale Instance Cleanup                                             |
| **Module**         | edgequake-core                                                     |
| **Status**         | ✅ Stable                                                          |
| **Code Reference** | [tenant_manager.rs](../edgequake/crates/edgequake-core/src/tenant_manager.rs) |
| **Description**    | Automatic cleanup of stale instances                               |
| **Related**        | FEAT0830                                                           |

---

## Advanced PDF Features (FEAT10XX)

> These features extend the basic PDF capabilities (FEAT05XX) with advanced extraction algorithms.

### FEAT1001 - PDF to Markdown Conversion

| Attribute          | Value                                                              |
| ------------------ | ------------------------------------------------------------------ |
| **ID**             | FEAT1001                                                           |
| **Name**           | Core PDF to Markdown Conversion                                    |
| **Module**         | edgequake-pdf                                                      |
| **Status**         | ✅ Stable                                                          |
| **Code Reference** | [extractor.rs](../edgequake/crates/edgequake-pdf/src/extractor.rs) |
| **Description**    | Convert PDF documents to Markdown with structure preservation      |
| **Related**        | FEAT0501, FEAT0504, FEAT1002-FEAT1006                              |

### FEAT1002 - Lattice Table Detection

| Attribute          | Value                                                                          |
| ------------------ | ------------------------------------------------------------------------------ |
| **ID**             | FEAT1002                                                                       |
| **Name**           | Lattice and Stream Table Detection                                             |
| **Module**         | edgequake-pdf                                                                  |
| **Status**         | ✅ Stable                                                                      |
| **Code Reference** | [backend/lattice.rs](../edgequake/crates/edgequake-pdf/src/backend/lattice.rs) |
| **Description**    | Detect tables using line-based (lattice) and whitespace-based (stream) modes   |
| **Related**        | FEAT0503, FEAT1001                                                             |

### FEAT1003 - Multi-Column Layout Detection

| Attribute          | Value                                                                      |
| ------------------ | -------------------------------------------------------------------------- |
| **ID**             | FEAT1003                                                                   |
| **Name**           | Multi-Column Layout Detection                                              |
| **Module**         | edgequake-pdf                                                              |
| **Status**         | ✅ Stable                                                                  |
| **Code Reference** | [layout/column_detector.rs](../edgequake/crates/edgequake-pdf/src/layout/) |
| **Description**    | Detect multi-column layouts and determine correct reading order            |
| **Related**        | FEAT0502, FEAT1001                                                         |

### FEAT1004 - Image Extraction with OCR

| Attribute          | Value                                                                            |
| ------------------ | -------------------------------------------------------------------------------- |
| **ID**             | FEAT1004                                                                         |
| **Name**           | Image Extraction with Optional OCR                                               |
| **Module**         | edgequake-pdf                                                                    |
| **Status**         | ✅ Stable                                                                        |
| **Code Reference** | [image_extraction.rs](../edgequake/crates/edgequake-pdf/src/image_extraction.rs) |
| **Description**    | Extract embedded images from PDF pages with optional OCR processing              |
| **Related**        | FEAT1023, FEAT1024                                                               |

### FEAT1005 - Formula Detection

| Attribute          | Value                                                      |
| ------------------ | ---------------------------------------------------------- |
| **ID**             | FEAT1005                                                   |
| **Name**           | Formula Detection and LaTeX Conversion                     |
| **Module**         | edgequake-pdf                                              |
| **Status**         | 🔧 Beta                                                    |
| **Code Reference** | [formula/](../edgequake/crates/edgequake-pdf/src/formula/) |
| **Description**    | Detect mathematical formulas and convert to LaTeX notation |
| **Related**        | FEAT1001, FEAT1004                                         |

### FEAT1006 - LLM-Enhanced Content Cleaning

| Attribute          | Value                                                              |
| ------------------ | ------------------------------------------------------------------ |
| **ID**             | FEAT1006                                                           |
| **Name**           | LLM-Enhanced Content Cleaning                                      |
| **Module**         | edgequake-pdf                                                      |
| **Status**         | 📋 Planned                                                         |
| **Code Reference** | [extractor.rs](../edgequake/crates/edgequake-pdf/src/extractor.rs) |
| **Description**    | Use LLM to clean and format extracted PDF content                  |
| **Related**        | FEAT1001, FEAT0003                                                 |

### FEAT1010 - Font Analysis

| Attribute          | Value                                                                                    |
| ------------------ | ---------------------------------------------------------------------------------------- |
| **ID**             | FEAT1010                                                                                 |
| **Name**           | Font Analysis and Classification                                                         |
| **Module**         | edgequake-pdf                                                                            |
| **Status**         | ✅ Stable                                                                                |
| **Code Reference** | [backend/sota_backend.rs](../edgequake/crates/edgequake-pdf/src/backend/sota_backend.rs) |
| **Description**    | Analyze font properties for heading detection and style classification                   |
| **Related**        | FEAT0505, FEAT1001                                                                       |

### FEAT1020 - Processor Pipeline

| Attribute          | Value                                                            |
| ------------------ | ---------------------------------------------------------------- |
| **ID**             | FEAT1020                                                         |
| **Name**           | Modular Processor Pipeline                                       |
| **Module**         | edgequake-pdf                                                    |
| **Status**         | ✅ Stable                                                        |
| **Code Reference** | [processors/](../edgequake/crates/edgequake-pdf/src/processors/) |
| **Description**    | Chainable processor pipeline for PDF content transformation      |
| **Related**        | FEAT1001, FEAT1021, FEAT1022                                     |

### FEAT1021 - Text Cleanup Processors

| Attribute          | Value                                                                                          |
| ------------------ | ---------------------------------------------------------------------------------------------- |
| **ID**             | FEAT1021                                                                                       |
| **Name**           | Text Cleanup Processors                                                                        |
| **Module**         | edgequake-pdf                                                                                  |
| **Status**         | ✅ Stable                                                                                      |
| **Code Reference** | [processors/text_cleanup.rs](../edgequake/crates/edgequake-pdf/src/processors/text_cleanup.rs) |
| **Description**    | Clean garbled text, fix hyphenation, filter whitespace                                         |
| **Related**        | FEAT1020, FEAT1001                                                                             |

### FEAT1022 - Structure Detection Processors

| Attribute          | Value                                                                                                        |
| ------------------ | ------------------------------------------------------------------------------------------------------------ |
| **ID**             | FEAT1022                                                                                                     |
| **Name**           | Structure Detection Processors                                                                               |
| **Module**         | edgequake-pdf                                                                                                |
| **Status**         | ✅ Stable                                                                                                    |
| **Code Reference** | [processors/structure_detection.rs](../edgequake/crates/edgequake-pdf/src/processors/structure_detection.rs) |
| **Description**    | Detect headers, captions, lists, and code blocks                                                             |
| **Related**        | FEAT1020, FEAT0505                                                                                           |

### FEAT1023 - Image Format Conversion

| Attribute          | Value                                                                            |
| ------------------ | -------------------------------------------------------------------------------- |
| **ID**             | FEAT1023                                                                         |
| **Name**           | Image Format Conversion                                                          |
| **Module**         | edgequake-pdf                                                                    |
| **Status**         | 📋 Planned                                                                       |
| **Code Reference** | [image_extraction.rs](../edgequake/crates/edgequake-pdf/src/image_extraction.rs) |
| **Description**    | Convert PDF images to PNG/JPEG for LLM processing                                |
| **Related**        | FEAT1004, FEAT1024                                                               |

### FEAT1024 - LLM-Based Image Understanding

| Attribute          | Value                                                              |
| ------------------ | ------------------------------------------------------------------ |
| **ID**             | FEAT1024                                                           |
| **Name**           | LLM-Based Image Understanding                                      |
| **Module**         | edgequake-pdf                                                      |
| **Status**         | ✅ Stable                                                          |
| **Code Reference** | [image_ocr.rs](../edgequake/crates/edgequake-pdf/src/image_ocr.rs) |
| **Description**    | Use vision LLM to extract text and understand image content        |
| **Related**        | FEAT1004, FEAT1023, FEAT1025                                       |

### FEAT1025 - Chart and Diagram Extraction

| Attribute          | Value                                                              |
| ------------------ | ------------------------------------------------------------------ |
| **ID**             | FEAT1025                                                           |
| **Name**           | Chart and Diagram Data Extraction                                  |
| **Module**         | edgequake-pdf                                                      |
| **Status**         | 🔧 Beta                                                            |
| **Code Reference** | [image_ocr.rs](../edgequake/crates/edgequake-pdf/src/image_ocr.rs) |
| **Description**    | Extract structured data from charts and diagrams using vision LLM  |
| **Related**        | FEAT1024, FEAT1004                                                 |

---

---

## Newly Discovered Features (Auto-Generated)

**Added**: 2026-01-09

### FEAT0206 - Graph Viewer functionality

**Module:** Graph Operations

**Source:** `edgequake_webui/src/components/graph/graph-viewer.tsx` (line 12)

**Status:** `ACTIVE`

---

### FEAT0540 - React Query hooks for lineage data fetching.

**Module:** LLM Integration

**Source:** `edgequake_webui/src/hooks/use-lineage.ts` (line 7)

**Status:** `ACTIVE`

---

### FEAT0541 - React Query hooks for lineage data fetching.

**Module:** LLM Integration

**Source:** `edgequake_webui/src/hooks/use-lineage.ts` (line 8)

**Status:** `ACTIVE`

---

### FEAT0583 - React Query hooks for folder management.

**Module:** LLM Integration

**Source:** `edgequake_webui/src/hooks/use-folders.ts` (line 9)

**Status:** `ACTIVE`

---

### FEAT0621 - Zustand store for backend connection and pipeline status.

**Module:** WebUI Core

**Source:** `edgequake_webui/src/stores/use-backend-store.ts` (line 8)

**Status:** `ACTIVE`

---

### FEAT0622 - Zustand store for UI layout preferences.

**Module:** WebUI Core

**Source:** `edgequake_webui/src/stores/use-ui-preferences-store.ts` (line 6)

**Status:** `ACTIVE`

---

### FEAT0623 - Zustand store for UI layout preferences.

**Module:** WebUI Core

**Source:** `edgequake_webui/src/stores/use-ui-preferences-store.ts` (line 7)

**Status:** `ACTIVE`

---

### FEAT0624 - Zustand store for UI layout preferences.

**Module:** WebUI Core

**Source:** `edgequake_webui/src/stores/use-ui-preferences-store.ts` (line 8)

**Status:** `ACTIVE`

---

### FEAT0625 - Types for real-time ingestion progress tracking.

**Module:** WebUI Core

**Source:** `edgequake_webui/src/types/ingestion.ts` (line 8)

**Additional locations:**

- `edgequake_webui/src/stores/use-query-ui-store.ts` (line 6)

**Status:** `ACTIVE`

---

### FEAT0626 - Zustand store for query UI state management.

**Module:** WebUI Core

**Source:** `edgequake_webui/src/stores/use-query-ui-store.ts` (line 7)

**Additional locations:**

- `edgequake_webui/src/components/graph/graph-search.tsx` (line 8)

**Status:** `ACTIVE`

---

### FEAT0627 - Zustand store for query UI state management.

**Module:** WebUI Core

**Source:** `edgequake_webui/src/stores/use-query-ui-store.ts` (line 8)

**Additional locations:**

- `edgequake_webui/src/components/graph/graph-filters.tsx` (line 8)

**Status:** `ACTIVE`

---

### FEAT0628 - React Query hooks for folder management.

**Module:** WebUI Core

**Source:** `edgequake_webui/src/hooks/use-folders.ts` (line 10)

**Additional locations:**

- `edgequake_webui/src/components/graph/entity-browser-panel.tsx` (line 7)

**Status:** `ACTIVE`

---

### FEAT0629 - Use Tenant Context functionality

**Module:** WebUI Core

**Source:** `edgequake_webui/src/hooks/use-tenant-context.ts` (line 12)

**Additional locations:**

- `edgequake_webui/src/components/graph/entity-browser-panel.tsx` (line 8)

**Status:** `ACTIVE`

---

### FEAT0630 - Hook for keyboard shortcut management.

**Module:** WebUI Core

**Source:** `edgequake_webui/src/hooks/use-keyboard-shortcuts.ts` (line 8)

**Additional locations:**

- `edgequake_webui/src/components/graph/entity-browser-panel.tsx` (line 9)

**Status:** `ACTIVE`

---

### FEAT0631 - Hook for keyboard shortcut management.

**Module:** WebUI Core

**Source:** `edgequake_webui/src/hooks/use-keyboard-shortcuts.ts` (line 9)

**Additional locations:**

- `edgequake_webui/src/components/documents/document-detail-dialog.tsx` (line 7)

**Status:** `ACTIVE`

---

### FEAT0632 - Hook for keyboard shortcut management.

**Module:** WebUI Core

**Source:** `edgequake_webui/src/hooks/use-keyboard-shortcuts.ts` (line 10)

**Additional locations:**

- `edgequake_webui/src/components/documents/document-detail-dialog.tsx` (line 8)

**Status:** `ACTIVE`

---

### FEAT0633 - Hook to auto-resize a textarea based on content

**Module:** WebUI Core

**Source:** `edgequake_webui/src/hooks/use-auto-resize.ts` (line 9)

**Additional locations:**

- `edgequake_webui/src/components/documents/document-preview-panel.tsx` (line 7)

**Status:** `ACTIVE`

---

### FEAT0634 - Hook to auto-resize a textarea based on content

**Module:** WebUI Core

**Source:** `edgequake_webui/src/hooks/use-auto-resize.ts` (line 10)

**Additional locations:**

- `edgequake_webui/src/components/documents/document-preview-panel.tsx` (line 8)

**Status:** `ACTIVE`

---

### FEAT0635 - useDebounce hook - Debounces a value by a specified delay.

**Module:** WebUI Core

**Source:** `edgequake_webui/src/hooks/use-debounce.ts` (line 12)

**Additional locations:**

- `edgequake_webui/src/components/documents/document-preview-panel.tsx` (line 9)

**Status:** `ACTIVE`

---

### FEAT0636 - Useful for search inputs where you want to wait for the user

**Module:** WebUI Core

**Source:** `edgequake_webui/src/hooks/use-debounce.ts` (line 13)

**Additional locations:**

- `edgequake_webui/src/components/shared/empty-state.tsx` (line 6)

**Status:** `ACTIVE`

---

### FEAT0637 - Hook for expanding graph nodes by fetching neighbors.

**Module:** WebUI Core

**Source:** `edgequake_webui/src/hooks/use-graph-expansion.ts` (line 9)

**Additional locations:**

- `edgequake_webui/src/components/shared/empty-state.tsx` (line 7)

**Status:** `ACTIVE`

---

### FEAT0638 - Hook for expanding graph nodes by fetching neighbors.

**Module:** WebUI Core

**Source:** `edgequake_webui/src/hooks/use-graph-expansion.ts` (line 10)

**Additional locations:**

- `edgequake_webui/src/components/shared/websocket-status.tsx` (line 7)

**Status:** `ACTIVE`

---

### FEAT0639 - Custom hook for keyboard navigation in the graph viewer.

**Module:** WebUI Core

**Source:** `edgequake_webui/src/hooks/use-graph-keyboard-navigation.ts` (line 9)

**Additional locations:**

- `edgequake_webui/src/components/shared/api-explorer.tsx` (line 7)

**Status:** `ACTIVE`

---

### FEAT0640 - Custom hook for keyboard navigation in the graph viewer.

**Module:** WebUI Core

**Source:** `edgequake_webui/src/hooks/use-graph-keyboard-navigation.ts` (line 10)

**Additional locations:**

- `edgequake_webui/src/components/shared/api-explorer.tsx` (line 8)

**Status:** `ACTIVE`

---

### FEAT0641 - Hook to check if a media query matches.

**Module:** WebUI Core

**Source:** `edgequake_webui/src/hooks/use-media-query.ts` (line 10)

**Status:** `ACTIVE`

---

### FEAT0642 - Hook to check if a media query matches.

**Module:** WebUI Core

**Source:** `edgequake_webui/src/hooks/use-media-query.ts` (line 11)

**Status:** `ACTIVE`

---

### FEAT0643 - Hook for migrating localStorage conversations to server.

**Module:** WebUI Core

**Source:** `edgequake_webui/src/hooks/use-migrate-conversations.ts` (line 9)

**Status:** `ACTIVE`

---

### FEAT0644 - Hook for migrating localStorage conversations to server.

**Module:** WebUI Core

**Source:** `edgequake_webui/src/hooks/use-migrate-conversations.ts` (line 10)

**Status:** `ACTIVE`

---

### FEAT0645 - Hook for managing query page state.

**Module:** WebUI Core

**Source:** `edgequake_webui/src/hooks/use-query-page-state.ts` (line 8)

**Status:** `ACTIVE`

---

### FEAT0646 - Hook for managing query page state.

**Module:** WebUI Core

**Source:** `edgequake_webui/src/hooks/use-query-page-state.ts` (line 9)

**Status:** `ACTIVE`

---

### FEAT0647 - A hook for syncing state with URL search parameters.

**Module:** WebUI Core

**Source:** `edgequake_webui/src/hooks/use-url-state.ts` (line 11)

**Status:** `ACTIVE`

---

### FEAT0648 - A hook for syncing state with URL search parameters.

**Module:** WebUI Core

**Source:** `edgequake_webui/src/hooks/use-url-state.ts` (line 12)

**Status:** `ACTIVE`

---

### FEAT0649 - Provides SSR-safe hooks for accessing Zustand persisted stores.

**Module:** WebUI Core

**Source:** `edgequake_webui/src/hooks/use-store-hydration.ts` (line 9)

**Status:** `ACTIVE`

---

### FEAT0650 - Provides SSR-safe hooks for accessing Zustand persisted stores.

**Module:** WebUI Core

**Source:** `edgequake_webui/src/hooks/use-store-hydration.ts` (line 10)

**Status:** `ACTIVE`

---

### FEAT0651 - Supports two URL formats:

**Module:** WebUI Core

**Source:** `edgequake_webui/src/hooks/use-workspace-url.ts` (line 12)

**Status:** `ACTIVE`

---

### FEAT0652 - Use Workspace Url functionality

**Module:** WebUI Core

**Source:** `edgequake_webui/src/hooks/use-workspace-url.ts` (line 13)

**Status:** `ACTIVE`

---

### FEAT0714 - Using raw graph coordinates directly in camera.animate() will cause the camera

**Module:** API Client

**Source:** `edgequake_webui/src/lib/graph/camera-utils.ts` (line 13)

**Status:** `ACTIVE`

---

### FEAT0715 - Using raw graph coordinates directly in camera.animate() will cause the camera

**Module:** API Client

**Source:** `edgequake_webui/src/lib/graph/camera-utils.ts` (line 14)

**Status:** `ACTIVE`

---

### FEAT0716 - Graph clustering using Louvain community detection.

**Module:** API Client

**Source:** `edgequake_webui/src/lib/graph/clustering.ts` (line 6)

**Status:** `ACTIVE`

---

### FEAT0717 - Graph clustering using Louvain community detection.

**Module:** API Client

**Source:** `edgequake_webui/src/lib/graph/clustering.ts` (line 7)

**Status:** `ACTIVE`

---

### FEAT0718 - Converts between API response format (SourceReference[]) and UI format (QueryContext).

**Module:** API Client

**Source:** `edgequake_webui/src/lib/utils/source-mapper.ts` (line 9)

**Status:** `ACTIVE`

---

### FEAT0719 - Converts between API response format (SourceReference[]) and UI format (QueryContext).

**Module:** API Client

**Source:** `edgequake_webui/src/lib/utils/source-mapper.ts` (line 10)

**Status:** `ACTIVE`

---

### FEAT0720 - UUID Generation Utilities

**Module:** API Client

**Source:** `edgequake_webui/src/lib/utils/uuid.ts` (line 8)

**Status:** `ACTIVE`

---

### FEAT0721 - Provides cross-browser compatible UUID v4 generation.

**Module:** API Client

**Source:** `edgequake_webui/src/lib/utils/uuid.ts` (line 9)

**Status:** `ACTIVE`

---

### FEAT0722 - WebSocket Manager Singleton

**Module:** API Client

**Source:** `edgequake_webui/src/lib/websocket/websocket-manager.ts` (line 8)

**Status:** `ACTIVE`

---

### FEAT0723 - Provides a single shared WebSocket connection for the application.

**Module:** API Client

**Source:** `edgequake_webui/src/lib/websocket/websocket-manager.ts` (line 9)

**Status:** `ACTIVE`

---

### FEAT0724 - WebSocket Client for Progress Tracking

**Module:** API Client

**Source:** `edgequake_webui/src/lib/websocket/progress-websocket.ts` (line 8)

**Additional locations:**

- `edgequake_webui/src/providers/websocket-provider.tsx` (line 6)

**Status:** `ACTIVE`

---

### FEAT0725 - Provides real-time progress updates for document ingestion.

**Module:** API Client

**Source:** `edgequake_webui/src/lib/websocket/progress-websocket.ts` (line 9)

**Status:** `ACTIVE`

---

### FEAT0726 - Provides real-time progress updates for document ingestion.

**Module:** API Client

**Source:** `edgequake_webui/src/lib/websocket/progress-websocket.ts` (line 10)

**Status:** `ACTIVE`

---

### FEAT0729 - Internationalization configuration.

**Module:** API Client

**Source:** `edgequake_webui/src/lib/i18n.ts` (line 6)

**Additional locations:**

- `edgequake_webui/src/app/layout.tsx` (line 7)
- `edgequake_webui/src/providers/i18n-provider.tsx` (line 5)

**Status:** `ACTIVE`

---

### FEAT0730 - Internationalization configuration.

**Module:** API Client

**Source:** `edgequake_webui/src/lib/i18n.ts` (line 7)

**Status:** `ACTIVE`

---

### FEAT0731 - Centralized Storage Keys

**Module:** API Client

**Source:** `edgequake_webui/src/lib/storage-keys.ts` (line 8)

**Status:** `ACTIVE`

---

### FEAT0732 - Single source of truth for all localStorage keys used in EdgeQuake WebUI.

**Module:** API Client

**Source:** `edgequake_webui/src/lib/storage-keys.ts` (line 9)

**Status:** `ACTIVE`

---

### FEAT0740 - Lists conversations, handles folder organization, and supports CRUD operations.

**Module:** API Client

**Source:** `edgequake_webui/src/components/query/conversation-history-panel.tsx` (line 9)

**Status:** `ACTIVE`

---

### FEAT0741 - Conversation History Panel functionality

**Module:** API Client

**Source:** `edgequake_webui/src/components/query/conversation-history-panel.tsx` (line 10)

**Status:** `ACTIVE`

---

### FEAT0750 - Chain-of-thought reasoning display component.

**Module:** API Client

**Source:** `edgequake_webui/src/components/query/thinking-display.tsx` (line 7)

**Status:** `ACTIVE`

---

### FEAT0751 - Graph layout and display controls.

**Module:** API Client

**Source:** `edgequake_webui/src/components/graph/graph-controls.tsx` (line 8)

**Status:** `ACTIVE`

---

### FEAT0760 - Real-time ingestion progress display with WebSocket support.

**Module:** API Client

**Source:** `edgequake_webui/src/components/documents/ingestion-progress-panel.tsx` (line 8)

**Status:** `ACTIVE`

---

### FEAT0770 - Base API client for EdgeQuake backend.

**Module:** API Client

**Source:** `edgequake_webui/src/lib/api/client.ts` (line 7)

**Status:** `ACTIVE`

---

### FEAT0771 - Base API client for EdgeQuake backend.

**Module:** API Client

**Source:** `edgequake_webui/src/lib/api/client.ts` (line 8)

**Status:** `ACTIVE`

---

### FEAT0772 - This module provides the client for the unified chat completions API.

**Module:** API Client

**Source:** `edgequake_webui/src/lib/api/chat.ts` (line 9)

**Status:** `ACTIVE`

---

### FEAT0773 - This module provides the client for the unified chat completions API.

**Module:** API Client

**Source:** `edgequake_webui/src/lib/api/chat.ts` (line 10)

**Status:** `ACTIVE`

---

### FEAT0774 - The server-side endpoint handles message persistence, so the client

**Module:** API Client

**Source:** `edgequake_webui/src/lib/api/chat.ts` (line 11)

**Status:** `ACTIVE`

---

### FEAT0800 - Root layout component for EdgeQuake WebUI.

**Module:** Authentication

**Source:** `edgequake_webui/src/app/layout.tsx` (line 6)

**Additional locations:**

- `edgequake_webui/src/providers/theme-provider.tsx` (line 5)

**Status:** `ACTIVE`

---

### FEAT0850 - Types for LLM cost monitoring and budget management.

**Module:** Cost Management

**Source:** `edgequake_webui/src/types/cost.ts` (line 8)

**Additional locations:**

- `edgequake_webui/src/stores/use-cost-store.ts` (line 8)
- `edgequake_webui/src/hooks/use-cost.ts` (line 8)

**Status:** `ACTIVE`

---

### FEAT0851 - Based on WebUI Specification Document WEBUI-007 (16-webui-cost-monitoring.md)

**Module:** Cost Management

**Source:** `edgequake_webui/src/stores/use-cost-store.ts` (line 9)

**Status:** `ACTIVE`

---

### FEAT0852 - Use Cost Store functionality

**Module:** Cost Management

**Source:** `edgequake_webui/src/stores/use-cost-store.ts` (line 10)

**Additional locations:**

- `edgequake_webui/src/hooks/use-cost.ts` (line 9)

**Status:** `ACTIVE`

---

### FEAT0853 - Based on WebUI Specification Document WEBUI-007 (16-webui-cost-monitoring.md)

**Module:** Cost Management

**Source:** `edgequake_webui/src/types/cost.ts` (line 9)

**Status:** `ACTIVE`

---

### FEAT0860 - Root provider composition for EdgeQuake WebUI.

**Module:** WebUI Providers

**Source:** `edgequake_webui/src/providers/index.tsx` (line 6)

**Status:** `ACTIVE`

---

### FEAT0861 - Zustand store for multi-tenant context management.

**Module:** WebUI Providers

**Source:** `edgequake_webui/src/stores/use-tenant-store.ts` (line 8)

**Additional locations:**

- `edgequake_webui/src/hooks/use-tenant-context.ts` (line 11)
- `edgequake_webui/src/providers/index.tsx` (line 7)
- `edgequake_webui/src/providers/tenant-provider.tsx` (line 6)
- `edgequake_webui/src/components/layout/header.tsx` (line 9)

**Status:** `ACTIVE`

---

### FEAT0862 - Manages tenant/workspace selection and provides API context headers.

**Module:** WebUI Providers

**Source:** `edgequake_webui/src/stores/use-tenant-store.ts` (line 9)

**Status:** `ACTIVE`

---

### FEAT0863 - React Query provider with default configuration.

**Module:** WebUI Providers

**Source:** `edgequake_webui/src/providers/query-provider.tsx` (line 5)

**Status:** `ACTIVE`

---

### FEAT0864 - React Query provider with default configuration.

**Module:** WebUI Providers

**Source:** `edgequake_webui/src/providers/query-provider.tsx` (line 6)

**Status:** `ACTIVE`

---

### FEAT0865 - WebSocket connection context for real-time progress tracking.

**Module:** WebUI Providers

**Source:** `edgequake_webui/src/providers/websocket-provider.tsx` (line 7)

**Status:** `ACTIVE`

---

### FEAT0867 - Internationalization provider with SSR-safe hydration.

**Module:** WebUI Providers

**Source:** `edgequake_webui/src/providers/i18n-provider.tsx` (line 6)

**Status:** `ACTIVE`

---

### FEAT0868 - Provider for tenant and workspace context initialization.

**Module:** WebUI Providers

**Source:** `edgequake_webui/src/providers/tenant-provider.tsx` (line 7)

**Status:** `ACTIVE`

---

### FEAT0870 - Index functionality

**Module:** Authentication

**Source:** `edgequake_webui/src/types/index.ts` (line 17)

**Additional locations:**

- `edgequake_webui/src/stores/use-auth-store.ts` (line 8)
- `edgequake_webui/src/lib/api/edgequake.ts` (line 9)

**Status:** `ACTIVE`

---

### FEAT0871 - Handles JWT tokens, user info, and login/logout actions.

**Module:** Authentication

**Source:** `edgequake_webui/src/stores/use-auth-store.ts` (line 9)

**Status:** `ACTIVE`

---

### FEAT0900 - Dashboard home page with stats, recent activity, and quick actions.

**Module:** Unknown

**Source:** `edgequake_webui/src/app/page.tsx` (line 5)

**Status:** `ACTIVE`

---

### FEAT0901 - Dashboard home page with stats, recent activity, and quick actions.

**Module:** Unknown

**Source:** `edgequake_webui/src/app/page.tsx` (line 6)

**Status:** `ACTIVE`

---

### FEAT0902 - Dashboard home page with stats, recent activity, and quick actions.

**Module:** Unknown

**Source:** `edgequake_webui/src/app/page.tsx` (line 7)

**Status:** `ACTIVE`

---

### FEAT1011 - Quick Actions functionality

**Module:** Document Management

**Source:** `edgequake_webui/src/components/dashboard/quick-actions.tsx` (line 5)

**Status:** `ACTIVE`

---

### FEAT1030 - System Status functionality

**Module:** Document Management

**Source:** `edgequake_webui/src/components/dashboard/system-status.tsx` (line 4)

**Status:** `ACTIVE`

---

### FEAT1031 - System Status functionality

**Module:** Document Management

**Source:** `edgequake_webui/src/components/dashboard/system-status.tsx` (line 5)

**Status:** `ACTIVE`

---

### FEAT1040 - Budget Indicator Component

**Module:** Document Management

**Source:** `edgequake_webui/src/components/cost/budget-indicator.tsx` (line 7)

**Status:** `ACTIVE`

---

### FEAT1041 - Visual indicator for budget status and limits.

**Module:** Document Management

**Source:** `edgequake_webui/src/components/cost/budget-indicator.tsx` (line 8)

**Status:** `ACTIVE`

---

### FEAT1042 - Cost Breakdown Chart Component

**Module:** Document Management

**Source:** `edgequake_webui/src/components/cost/cost-breakdown-chart.tsx` (line 7)

**Status:** `ACTIVE`

---

### FEAT1043 - Visual cost breakdown with pie or bar chart.

**Module:** Document Management

**Source:** `edgequake_webui/src/components/cost/cost-breakdown-chart.tsx` (line 8)

**Status:** `ACTIVE`

---

### FEAT1044 - Cost Summary Card Component

**Module:** Document Management

**Source:** `edgequake_webui/src/components/cost/cost-summary-card.tsx` (line 7)

**Status:** `ACTIVE`

---

### FEAT1045 - Overview card displaying cost summary statistics.

**Module:** Document Management

**Source:** `edgequake_webui/src/components/cost/cost-summary-card.tsx` (line 8)

**Status:** `ACTIVE`

---

### FEAT1046 - Token Usage Table Component

**Module:** Document Management

**Source:** `edgequake_webui/src/components/cost/token-usage-table.tsx` (line 7)

**Status:** `ACTIVE`

---

### FEAT1047 - Detailed table showing token usage by operation.

**Module:** Document Management

**Source:** `edgequake_webui/src/components/cost/token-usage-table.tsx` (line 8)

**Status:** `ACTIVE`

---

### FEAT1050 - Tour Provider functionality

**Module:** Document Management

**Source:** `edgequake_webui/src/components/onboarding/tour-provider.tsx` (line 4)

**Status:** `ACTIVE`

---

### FEAT1051 - Tour Provider functionality

**Module:** Document Management

**Source:** `edgequake_webui/src/components/onboarding/tour-provider.tsx` (line 5)

**Status:** `ACTIVE`

---

### FEAT1052 - Tour Steps functionality

**Module:** Document Management

**Source:** `edgequake_webui/src/components/onboarding/tour-steps.tsx` (line 4)

**Status:** `ACTIVE`

---

### FEAT1053 - Tour Steps functionality

**Module:** Document Management

**Source:** `edgequake_webui/src/components/onboarding/tour-steps.tsx` (line 5)

**Status:** `ACTIVE`

---

### FEAT1060 - Stage Indicator Component

**Module:** Document Management

**Source:** `edgequake_webui/src/components/progress/stage-indicator.tsx` (line 7)

**Status:** `ACTIVE`

---

### FEAT1061 - Pipeline stage visualization with progress.

**Module:** Document Management

**Source:** `edgequake_webui/src/components/progress/stage-indicator.tsx` (line 8)

**Status:** `ACTIVE`

---

### FEAT1062 - ETA Display Component

**Module:** Document Management

**Source:** `edgequake_webui/src/components/progress/eta-display.tsx` (line 7)

**Status:** `ACTIVE`

---

### FEAT1063 - Shows estimated time remaining for ingestion.

**Module:** Document Management

**Source:** `edgequake_webui/src/components/progress/eta-display.tsx` (line 8)

**Status:** `ACTIVE`

---

### FEAT1064 - Live Message Component

**Module:** Document Management

**Source:** `edgequake_webui/src/components/progress/live-message.tsx` (line 7)

**Status:** `ACTIVE`

---

### FEAT1065 - Displays streaming/live ingestion messages.

**Module:** Document Management

**Source:** `edgequake_webui/src/components/progress/live-message.tsx` (line 8)

**Status:** `ACTIVE`

---

### FEAT1070 - Chunk Explorer Component

**Module:** Document Management

**Source:** `edgequake_webui/src/components/document/chunk-explorer.tsx` (line 7)

**Status:** `ACTIVE`

---

### FEAT1071 - Browse document chunks with entity highlighting.

**Module:** Document Management

**Source:** `edgequake_webui/src/components/document/chunk-explorer.tsx` (line 8)

**Status:** `ACTIVE`

---

### FEAT1072 - Content Renderer functionality

**Module:** Document Management

**Source:** `edgequake_webui/src/components/document/content-renderer.tsx` (line 4)

**Status:** `ACTIVE`

---

### FEAT1073 - Content Renderer functionality

**Module:** Document Management

**Source:** `edgequake_webui/src/components/document/content-renderer.tsx` (line 5)

**Status:** `ACTIVE`

---

### FEAT1074 - Metadata Sidebar functionality

**Module:** Document Management

**Source:** `edgequake_webui/src/components/document/metadata-sidebar.tsx` (line 4)

**Status:** `ACTIVE`

---

### FEAT1075 - Metadata Sidebar functionality

**Module:** Document Management

**Source:** `edgequake_webui/src/components/document/metadata-sidebar.tsx` (line 5)

**Status:** `ACTIVE`

---

### FEAT1076 - Lineage Tree functionality

**Module:** Document Management

**Source:** `edgequake_webui/src/components/document/lineage-tree.tsx` (line 4)

**Status:** `ACTIVE`

---

### FEAT1077 - Lineage Tree functionality

**Module:** Document Management

**Source:** `edgequake_webui/src/components/document/lineage-tree.tsx` (line 5)

**Status:** `ACTIVE`

---

### FEAT1078 - Entity Relation Stats functionality

**Module:** Document Management

**Source:** `edgequake_webui/src/components/document/entity-relation-stats.tsx` (line 4)

**Status:** `ACTIVE`

---

### FEAT1079 - Entity Relation Stats functionality

**Module:** Document Management

**Source:** `edgequake_webui/src/components/document/entity-relation-stats.tsx` (line 5)

**Status:** `ACTIVE`

---

### FEAT1080 - Chunk Detail Modal Component

**Module:** Document Management

**Source:** `edgequake_webui/src/components/document/chunk-detail-modal.tsx` (line 7)

**Status:** `ACTIVE`

---

### FEAT1081 - Full chunk view with entities and relationships.

**Module:** Document Management

**Source:** `edgequake_webui/src/components/document/chunk-detail-modal.tsx` (line 8)

**Status:** `ACTIVE`

---

### FEAT1082 - Processing Details functionality

**Module:** Document Management

**Source:** `edgequake_webui/src/components/document/processing-details.tsx` (line 4)

**Status:** `ACTIVE`

---

### FEAT1083 - Processing Details functionality

**Module:** Document Management

**Source:** `edgequake_webui/src/components/document/processing-details.tsx` (line 5)

**Status:** `ACTIVE`

---

### FEAT1084 - Source Info Grid functionality

**Module:** Document Management

**Source:** `edgequake_webui/src/components/document/source-info-grid.tsx` (line 4)

**Status:** `ACTIVE`

---

### FEAT1085 - Source Info Grid functionality

**Module:** Document Management

**Source:** `edgequake_webui/src/components/document/source-info-grid.tsx` (line 5)

**Status:** `ACTIVE`

---

### FEAT1086 - Key Stats functionality

**Module:** Document Management

**Source:** `edgequake_webui/src/components/document/key-stats.tsx` (line 4)

**Status:** `ACTIVE`

---

### FEAT1087 - Key Stats functionality

**Module:** Document Management

**Source:** `edgequake_webui/src/components/document/key-stats.tsx` (line 5)

**Status:** `ACTIVE`

---

## Summary Statistics

| Category         | Total   | Stable  | Beta    | Planned |
| ---------------- | ------- | ------- | ------- | ------- |
| Core RAG         | 20      | 20      | 0       | 0       |
| Query Engine     | 10      | 10      | 0       | 0       |
| Storage          | 5       | 5       | 0       | 0       |
| Pipeline         | 4       | 4       | 0       | 0       |
| API              | 6       | 6       | 0       | 0       |
| PDF (Basic)      | 5       | 5       | 0       | 0       |
| PDF (Advanced)   | 14      | 12      | 2       | 0       |
| WebUI            | 20      | 19      | 0       | 1       |
| WebUI API Client | 17      | 17      | 0       | 0       |
| Auth             | 3       | 3       | 0       | 0       |
| **TOTAL**        | **224** | **224** | **224** | **224** |

---

## Related Documents

- [Business Rules](business_rules.md)
- [Use Cases](use_cases.md)
- [Architecture Overview](0002-architecture-overview.md)
- [API Reference](0003-api-reference.md)
- [WebUI Architecture](0011-webui-architecture.md)
- [WebUI State Management](0014-webui-state-management.md)

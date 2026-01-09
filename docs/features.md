# EdgeQuake Feature Registry

> Central registry of all features in EdgeQuake RAG system.
> Use FEATXXXX references in code comments for traceability.

**Version**: 1.1.0 | **Last Updated**: 2026-01-09

---

## Quick Reference Index

| Category                                                       | ID Range          | Count |
| -------------------------------------------------------------- | ----------------- | ----- |
| [Core RAG Features](#core-rag-features-feat00xx)               | FEAT0001-FEAT0020 | 20    |
| [Query Engine Features](#query-engine-features-feat01xx)       | FEAT0101-FEAT0120 | 10    |
| [Storage Features](#storage-features-feat02xx)                 | FEAT0201-FEAT0220 | 5     |
| [Pipeline Features](#pipeline-features-feat03xx)               | FEAT0301-FEAT0320 | 4     |
| [API Features](#api-features-feat04xx)                         | FEAT0401-FEAT0420 | 6     |
| [PDF Features](#pdf-features-feat05xx)                         | FEAT0501-FEAT0520 | 5     |
| [Advanced PDF Features](#advanced-pdf-features-feat10xx)       | FEAT1001-FEAT1025 | 14    |
| [WebUI Features](#webui-features-feat06xx)                     | FEAT0601-FEAT0620 | 4     |
| [Auth Features](#auth-features-feat07xx)                       | FEAT0701-FEAT0720 | 3     |

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
| **Status**         | ✅ Stable                                                                          |
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
| **Status**         | ✅ Stable                                                                    |
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

### FEAT0601 - Document Upload UI

| Attribute          | Value                                                    |
| ------------------ | -------------------------------------------------------- |
| **ID**             | FEAT0601                                                 |
| **Name**           | Document Upload Interface                                |
| **Module**         | edgequake_webui                                          |
| **Status**         | ✅ Stable                                                |
| **Code Reference** | [components/upload/](../edgequake_webui/src/components/) |
| **Description**    | Drag-and-drop file upload interface                      |
| **Related**        | FEAT0401, FEAT0402                                       |

### FEAT0602 - Chat Interface

| Attribute          | Value                                    |
| ------------------ | ---------------------------------------- |
| **ID**             | FEAT0602                                 |
| **Name**           | Chat Query Interface                     |
| **Module**         | edgequake_webui                          |
| **Status**         | ✅ Stable                                |
| **Code Reference** | [app/chat/](../edgequake_webui/src/app/) |
| **Description**    | Conversational interface for RAG queries |
| **Related**        | FEAT0403, FEAT0404                       |

### FEAT0603 - Graph Visualization

| Attribute          | Value                                                   |
| ------------------ | ------------------------------------------------------- |
| **ID**             | FEAT0603                                                |
| **Name**           | Knowledge Graph Visualization                           |
| **Module**         | edgequake_webui                                         |
| **Status**         | ✅ Stable                                               |
| **Code Reference** | [components/graph/](../edgequake_webui/src/components/) |
| **Description**    | Interactive graph visualization using Sigma.js          |
| **Related**        | FEAT0405, UC0101                                        |

### FEAT0604 - Workspace Switcher

| Attribute          | Value                                                       |
| ------------------ | ----------------------------------------------------------- |
| **ID**             | FEAT0604                                                    |
| **Name**           | Workspace Selector UI                                       |
| **Module**         | edgequake_webui                                             |
| **Status**         | ✅ Stable                                                   |
| **Code Reference** | [components/workspace/](../edgequake_webui/src/components/) |
| **Description**    | Switch between workspaces in the UI                         |
| **Related**        | FEAT0016, UC0301                                            |

---

## Auth Features (FEAT07XX)

### FEAT0701 - API Key Authentication

| Attribute          | Value                                                                 |
| ------------------ | --------------------------------------------------------------------- |
| **ID**             | FEAT0701                                                              |
| **Name**           | API Key Authentication                                                |
| **Module**         | edgequake-auth                                                        |
| **Status**         | ✅ Stable                                                             |
| **Code Reference** | [extractors.rs](../edgequake/crates/edgequake-auth/src/extractors.rs) |
| **Description**    | Authenticate requests via API key header                              |
| **Related**        | BR0201, FEAT0015                                                      |

### FEAT0702 - JWT Token Support

| Attribute          | Value                                                   |
| ------------------ | ------------------------------------------------------- |
| **ID**             | FEAT0702                                                |
| **Name**           | JWT Token Authentication                                |
| **Module**         | edgequake-auth                                          |
| **Status**         | ✅ Stable                                               |
| **Code Reference** | [jwt.rs](../edgequake/crates/edgequake-auth/src/jwt.rs) |
| **Description**    | Support JWT tokens for web client authentication        |
| **Related**        | FEAT0701, BR0201                                        |

### FEAT0703 - Role-Based Access Control

| Attribute          | Value                                                     |
| ------------------ | --------------------------------------------------------- |
| **ID**             | FEAT0703                                                  |
| **Name**           | RBAC Authorization                                        |
| **Module**         | edgequake-auth                                            |
| **Status**         | ✅ Stable                                                 |
| **Code Reference** | [rbac.rs](../edgequake/crates/edgequake-auth/src/rbac.rs) |
| **Description**    | Role-based permission checks                              |
| **Related**        | BR0202, FEAT0701                                          |

---

## Advanced PDF Features (FEAT10XX)

> These features extend the basic PDF capabilities (FEAT05XX) with advanced extraction algorithms.

### FEAT1001 - PDF to Markdown Conversion

| Attribute          | Value                                                                   |
| ------------------ | ----------------------------------------------------------------------- |
| **ID**             | FEAT1001                                                                |
| **Name**           | Core PDF to Markdown Conversion                                         |
| **Module**         | edgequake-pdf                                                           |
| **Status**         | ✅ Stable                                                               |
| **Code Reference** | [extractor.rs](../edgequake/crates/edgequake-pdf/src/extractor.rs)      |
| **Description**    | Convert PDF documents to Markdown with structure preservation           |
| **Related**        | FEAT0501, FEAT0504, FEAT1002-FEAT1006                                   |

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

| Attribute          | Value                                                                            |
| ------------------ | -------------------------------------------------------------------------------- |
| **ID**             | FEAT1003                                                                         |
| **Name**           | Multi-Column Layout Detection                                                    |
| **Module**         | edgequake-pdf                                                                    |
| **Status**         | ✅ Stable                                                                        |
| **Code Reference** | [layout/column_detector.rs](../edgequake/crates/edgequake-pdf/src/layout/)       |
| **Description**    | Detect multi-column layouts and determine correct reading order                  |
| **Related**        | FEAT0502, FEAT1001                                                               |

### FEAT1004 - Image Extraction with OCR

| Attribute          | Value                                                                                 |
| ------------------ | ------------------------------------------------------------------------------------- |
| **ID**             | FEAT1004                                                                              |
| **Name**           | Image Extraction with Optional OCR                                                    |
| **Module**         | edgequake-pdf                                                                         |
| **Status**         | ✅ Stable                                                                             |
| **Code Reference** | [image_extraction.rs](../edgequake/crates/edgequake-pdf/src/image_extraction.rs)      |
| **Description**    | Extract embedded images from PDF pages with optional OCR processing                   |
| **Related**        | FEAT1023, FEAT1024                                                                    |

### FEAT1005 - Formula Detection

| Attribute          | Value                                                              |
| ------------------ | ------------------------------------------------------------------ |
| **ID**             | FEAT1005                                                           |
| **Name**           | Formula Detection and LaTeX Conversion                             |
| **Module**         | edgequake-pdf                                                      |
| **Status**         | 🔧 Beta                                                            |
| **Code Reference** | [formula/](../edgequake/crates/edgequake-pdf/src/formula/)         |
| **Description**    | Detect mathematical formulas and convert to LaTeX notation         |
| **Related**        | FEAT1001, FEAT1004                                                 |

### FEAT1006 - LLM-Enhanced Content Cleaning

| Attribute          | Value                                                                   |
| ------------------ | ----------------------------------------------------------------------- |
| **ID**             | FEAT1006                                                                |
| **Name**           | LLM-Enhanced Content Cleaning                                           |
| **Module**         | edgequake-pdf                                                           |
| **Status**         | ✅ Stable                                                               |
| **Code Reference** | [extractor.rs](../edgequake/crates/edgequake-pdf/src/extractor.rs)      |
| **Description**    | Use LLM to clean and format extracted PDF content                       |
| **Related**        | FEAT1001, FEAT0003                                                      |

### FEAT1010 - Font Analysis

| Attribute          | Value                                                                              |
| ------------------ | ---------------------------------------------------------------------------------- |
| **ID**             | FEAT1010                                                                           |
| **Name**           | Font Analysis and Classification                                                   |
| **Module**         | edgequake-pdf                                                                      |
| **Status**         | ✅ Stable                                                                          |
| **Code Reference** | [backend/sota_backend.rs](../edgequake/crates/edgequake-pdf/src/backend/sota_backend.rs) |
| **Description**    | Analyze font properties for heading detection and style classification             |
| **Related**        | FEAT0505, FEAT1001                                                                 |

### FEAT1020 - Processor Pipeline

| Attribute          | Value                                                                              |
| ------------------ | ---------------------------------------------------------------------------------- |
| **ID**             | FEAT1020                                                                           |
| **Name**           | Modular Processor Pipeline                                                         |
| **Module**         | edgequake-pdf                                                                      |
| **Status**         | ✅ Stable                                                                          |
| **Code Reference** | [processors/](../edgequake/crates/edgequake-pdf/src/processors/)                   |
| **Description**    | Chainable processor pipeline for PDF content transformation                        |
| **Related**        | FEAT1001, FEAT1021, FEAT1022                                                       |

### FEAT1021 - Text Cleanup Processors

| Attribute          | Value                                                                              |
| ------------------ | ---------------------------------------------------------------------------------- |
| **ID**             | FEAT1021                                                                           |
| **Name**           | Text Cleanup Processors                                                            |
| **Module**         | edgequake-pdf                                                                      |
| **Status**         | ✅ Stable                                                                          |
| **Code Reference** | [processors/text_cleanup.rs](../edgequake/crates/edgequake-pdf/src/processors/text_cleanup.rs) |
| **Description**    | Clean garbled text, fix hyphenation, filter whitespace                             |
| **Related**        | FEAT1020, FEAT1001                                                                 |

### FEAT1022 - Structure Detection Processors

| Attribute          | Value                                                                              |
| ------------------ | ---------------------------------------------------------------------------------- |
| **ID**             | FEAT1022                                                                           |
| **Name**           | Structure Detection Processors                                                     |
| **Module**         | edgequake-pdf                                                                      |
| **Status**         | ✅ Stable                                                                          |
| **Code Reference** | [processors/structure_detection.rs](../edgequake/crates/edgequake-pdf/src/processors/structure_detection.rs) |
| **Description**    | Detect headers, captions, lists, and code blocks                                   |
| **Related**        | FEAT1020, FEAT0505                                                                 |

### FEAT1023 - Image Format Conversion

| Attribute          | Value                                                                              |
| ------------------ | ---------------------------------------------------------------------------------- |
| **ID**             | FEAT1023                                                                           |
| **Name**           | Image Format Conversion                                                            |
| **Module**         | edgequake-pdf                                                                      |
| **Status**         | ✅ Stable                                                                          |
| **Code Reference** | [image_extraction.rs](../edgequake/crates/edgequake-pdf/src/image_extraction.rs)   |
| **Description**    | Convert PDF images to PNG/JPEG for LLM processing                                  |
| **Related**        | FEAT1004, FEAT1024                                                                 |

### FEAT1024 - LLM-Based Image Understanding

| Attribute          | Value                                                                              |
| ------------------ | ---------------------------------------------------------------------------------- |
| **ID**             | FEAT1024                                                                           |
| **Name**           | LLM-Based Image Understanding                                                      |
| **Module**         | edgequake-pdf                                                                      |
| **Status**         | ✅ Stable                                                                          |
| **Code Reference** | [image_ocr.rs](../edgequake/crates/edgequake-pdf/src/image_ocr.rs)                 |
| **Description**    | Use vision LLM to extract text and understand image content                        |
| **Related**        | FEAT1004, FEAT1023, FEAT1025                                                       |

### FEAT1025 - Chart and Diagram Extraction

| Attribute          | Value                                                                              |
| ------------------ | ---------------------------------------------------------------------------------- |
| **ID**             | FEAT1025                                                                           |
| **Name**           | Chart and Diagram Data Extraction                                                  |
| **Module**         | edgequake-pdf                                                                      |
| **Status**         | 🔧 Beta                                                                            |
| **Code Reference** | [image_ocr.rs](../edgequake/crates/edgequake-pdf/src/image_ocr.rs)                 |
| **Description**    | Extract structured data from charts and diagrams using vision LLM                  |
| **Related**        | FEAT1024, FEAT1004                                                                 |

---

## Summary Statistics

| Category       | Total  | Stable | Beta  | Planned |
| -------------- | ------ | ------ | ----- | ------- |
| Core RAG       | 20     | 20     | 0     | 0       |
| Query Engine   | 10     | 10     | 0     | 0       |
| Storage        | 5      | 5      | 0     | 0       |
| Pipeline       | 4      | 4      | 0     | 0       |
| API            | 6      | 6      | 0     | 0       |
| PDF (Basic)    | 5      | 5      | 0     | 0       |
| PDF (Advanced) | 14     | 12     | 2     | 0       |
| WebUI          | 4      | 4      | 0     | 0       |
| Auth           | 3      | 3      | 0     | 0       |
| **TOTAL**      | **71** | **69** | **2** | **0**   |

---

## Related Documents

- [Business Rules](business_rules.md)
- [Use Cases](use_cases.md)
- [Architecture Overview](0002-architecture-overview.md)
- [API Reference](0003-api-reference.md)

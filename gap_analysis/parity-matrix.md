# Feature Parity Matrix

## Legend

- ✅ Full parity
- ⚠️ Partial implementation
- ❌ Not implemented
- 🔄 Different approach (functionally equivalent)
- ⬆️ Target exceeds source
- ➖ Not applicable

---

## Matrix by Category

### CORE: Core RAG Functionality

| ID    | Feature                   | Source | Target | Status | Gap ID  | Priority | Notes             |
| ----- | ------------------------- | ------ | ------ | ------ | ------- | -------- | ----------------- |
| F-001 | Document Insert (sync)    | ✅     | ✅     | ✅     | -       | -        | Equivalent        |
| F-002 | Document Insert (async)   | ✅     | ✅     | ✅     | -       | -        | Both async-first  |
| F-003 | Batch Document Insert     | ✅     | ✅     | ✅     | -       | -        | -                 |
| F-004 | Token-based Chunking      | ✅     | ✅     | ✅     | -       | -        | Uses tiktoken     |
| F-005 | Chunk Overlap             | ✅     | ✅     | ✅     | -       | -        | -                 |
| F-006 | Custom Chunking Function  | ✅     | ❌     | ❌     | GAP-016 | P2       | Not pluggable     |
| F-007 | Split by Character        | ✅     | ❌     | ❌     | GAP-017 | P3       | Pre-split option  |
| F-008 | Entity Extraction (LLM)   | ✅     | ✅     | ✅     | -       | -        | Similar prompts   |
| F-009 | Relationship Extraction   | ✅     | ✅     | ✅     | -       | -        | -                 |
| F-010 | Entity Deduplication      | ✅     | ⚠️     | ⚠️     | GAP-005 | P1       | Missing LLM merge |
| F-011 | Description Summarization | ✅     | ⚠️     | ⚠️     | GAP-006 | P1       | No map-reduce     |
| F-012 | Max Gleaning              | ✅     | ❌     | ❌     | GAP-018 | P2       | Re-extraction     |
| F-013 | Query Mode: Naive         | ✅     | ✅     | ✅     | -       | -        | Chunk retrieval   |
| F-014 | Query Mode: Local         | ✅     | ✅     | ✅     | -       | -        | Entity-centric    |
| F-015 | Query Mode: Global        | ✅     | ❌     | ❌     | GAP-001 | P0       | **Critical**      |
| F-016 | Query Mode: Hybrid        | ✅     | ⚠️     | ⚠️     | GAP-019 | P1       | Local+Global      |
| F-017 | Query Mode: Mix           | ✅     | ❌     | ❌     | GAP-002 | P0       | **Critical**      |
| F-018 | Query Mode: Bypass        | ✅     | ❌     | ❌     | GAP-020 | P3       | Direct LLM        |
| F-019 | Streaming Query           | ✅     | ✅     | ✅     | -       | -        | SSE/NDJSON        |
| F-020 | Conversation History      | ✅     | ✅     | ✅     | -       | -        | Multi-turn        |
| F-021 | Keyword Extraction        | ✅     | ❌     | ❌     | GAP-007 | P1       | HL/LL keywords    |
| F-022 | Context-only Query        | ✅     | ✅     | ✅     | -       | -        | No generation     |
| F-023 | Prompt-only Query         | ✅     | ❌     | ❌     | GAP-021 | P3       | Debug mode        |
| F-024 | Reference List            | ✅     | ⚠️     | ⚠️     | GAP-022 | P2       | Partial impl      |
| F-025 | Reranking                 | ✅     | ⚠️     | ⚠️     | GAP-008 | P1       | No real reranker  |
| F-026 | Token Budget              | ✅     | ⚠️     | ⚠️     | GAP-009 | P1       | Partial limits    |

---

### DATA: Storage & Data

| ID    | Feature              | Source | Target | Status | Gap ID  | Priority | Notes           |
| ----- | -------------------- | ------ | ------ | ------ | ------- | -------- | --------------- |
| F-027 | KV Storage Trait     | ✅     | ✅     | ✅     | -       | -        | Interface match |
| F-028 | Vector Storage Trait | ✅     | ✅     | ✅     | -       | -        | -               |
| F-029 | Graph Storage Trait  | ✅     | ✅     | ✅     | -       | -        | -               |
| F-030 | Document Status      | ✅     | ⚠️     | ⚠️     | GAP-023 | P2       | Missing fields  |
| F-031 | Memory Storage       | ✅     | ✅     | ✅     | -       | -        | Testing         |
| F-032 | PostgreSQL           | ✅     | ✅     | ✅     | -       | -        | Production      |
| F-033 | Neo4j                | ✅     | ❌     | ❌     | GAP-012 | P2       | Graph DB        |
| F-034 | Redis                | ✅     | ❌     | ❌     | GAP-024 | P3       | Cache/KV        |
| F-035 | MongoDB              | ✅     | ❌     | ❌     | GAP-025 | P3       | Document DB     |
| F-036 | Milvus/Qdrant        | ✅     | ❌     | ❌     | GAP-013 | P2       | Vector DB       |
| F-037 | FAISS                | ✅     | ❌     | ❌     | GAP-026 | P3       | Local vector    |
| F-038 | NanoVectorDB         | ✅     | ❌     | ❌     | GAP-027 | P3       | Lightweight     |

---

### INTG: External Integrations

| ID    | Feature         | Source | Target | Status | Gap ID  | Priority | Notes        |
| ----- | --------------- | ------ | ------ | ------ | ------- | -------- | ------------ |
| F-039 | OpenAI Provider | ✅     | ✅     | ✅     | -       | -        | Full support |
| F-040 | Anthropic       | ✅     | ❌     | ❌     | GAP-010 | P1       | Claude       |
| F-041 | Azure OpenAI    | ✅     | ❌     | ❌     | GAP-028 | P2       | Enterprise   |
| F-042 | Ollama          | ✅     | ⚠️     | ⚠️     | GAP-029 | P2       | Partial impl |
| F-043 | Gemini          | ✅     | ❌     | ❌     | GAP-030 | P2       | Google       |
| F-044 | Bedrock         | ✅     | ❌     | ❌     | GAP-031 | P2       | AWS          |
| F-045 | HuggingFace     | ✅     | ❌     | ❌     | GAP-032 | P3       | Local models |
| F-046 | Jina Embedding  | ✅     | ❌     | ❌     | GAP-033 | P2       | Embedding    |

---

### SEC: Security & Multi-Tenancy

| ID    | Feature             | Source | Target | Status | Gap ID  | Priority | Notes        |
| ----- | ------------------- | ------ | ------ | ------ | ------- | -------- | ------------ |
| F-066 | Multi-tenancy       | ✅     | ⚠️     | ⚠️     | GAP-003 | P0       | **Critical** |
| F-067 | Tenant RAG Manager  | ✅     | ❌     | ❌     | GAP-004 | P0       | **Critical** |
| F-068 | Tenant/KB Isolation | ✅     | ⚠️     | ⚠️     | GAP-037 | P0       | Partial      |
| F-069 | JWT Auth            | ✅     | ✅     | ✅     | -       | -        | Full support |
| F-070 | API Key Auth        | ✅     | ✅     | ✅     | -       | -        | -            |
| F-071 | RBAC                | ✅     | ✅     | ✅     | -       | -        | -            |

---

### API: REST API Surface

| ID    | Feature           | Source | Target | Status | Gap ID  | Priority | Notes           |
| ----- | ----------------- | ------ | ------ | ------ | ------- | -------- | --------------- |
| F-051 | Document Upload   | ✅     | ✅     | ✅     | -       | -        | -               |
| F-052 | File Upload       | ✅     | ✅     | ✅     | -       | -        | Multipart       |
| F-053 | Batch Upload      | ✅     | ✅     | ✅     | -       | -        | -               |
| F-054 | Document List     | ✅     | ✅     | ✅     | -       | -        | -               |
| F-055 | Document Delete   | ✅     | ✅     | ✅     | -       | -        | -               |
| F-056 | Track Status      | ✅     | ✅     | ✅     | -       | -        | -               |
| F-057 | Query             | ✅     | ✅     | ✅     | -       | -        | -               |
| F-058 | Stream Query      | ✅     | ✅     | ✅     | -       | -        | -               |
| F-059 | Graph Labels      | ✅     | ⚠️     | ⚠️     | GAP-036 | P2       | Missing popular |
| F-060 | Graph Knowledge   | ✅     | ✅     | ✅     | -       | -        | -               |
| F-061 | Entity CRUD       | ✅     | ✅     | ✅     | -       | -        | -               |
| F-062 | Relationship CRUD | ✅     | ✅     | ✅     | -       | -        | -               |
| F-063 | Entity Merge      | ✅     | ✅     | ✅     | -       | -        | -               |
| F-064 | Pipeline Status   | ✅     | ✅     | ✅     | -       | -        | -               |
| F-065 | Pipeline Cancel   | ✅     | ✅     | ✅     | -       | -        | -               |
| F-072 | Ollama Emulation  | ✅     | ❌     | ❌     | GAP-038 | P3       | Compat API      |
| F-073 | Health Check      | ✅     | ✅     | ✅     | -       | -        | -               |
| F-076 | Document Scan     | ✅     | ❌     | ❌     | GAP-014 | P2       | Dir scanning    |
| F-077 | Reprocess Failed  | ✅     | ❌     | ❌     | GAP-039 | P2       | Retry logic     |

---

### PERF: Performance Features

| ID    | Feature         | Source | Target | Status | Gap ID  | Priority | Notes        |
| ----- | --------------- | ------ | ------ | ------ | ------- | -------- | ------------ |
| F-047 | LLM Cache       | ✅     | ⚠️     | ⚠️     | GAP-015 | P1       | Partial      |
| F-048 | Embedding Cache | ✅     | ❌     | ❌     | GAP-034 | P2       | -            |
| F-049 | Rate Limiting   | ✅     | ❌     | ❌     | GAP-011 | P1       | Async limits |
| F-050 | Priority Queue  | ✅     | ❌     | ❌     | GAP-035 | P2       | LLM priority |

---

### OBS: Observability

| ID    | Feature         | Source | Target | Status | Gap ID | Priority | Notes         |
| ----- | --------------- | ------ | ------ | ------ | ------ | -------- | ------------- |
| F-074 | Metrics API     | ⚠️     | ✅     | ⬆️     | -      | P4       | Target better |
| F-075 | Tracing/Logging | ✅     | ✅     | ✅     | -      | -        | tracing crate |

---

### INTG: Document Processing

| ID    | Feature | Source | Target | Status | Gap ID  | Priority | Notes       |
| ----- | ------- | ------ | ------ | ------ | ------- | -------- | ----------- |
| F-078 | Docling | ✅     | ❌     | ❌     | GAP-040 | P3       | PDF parsing |

---

## Summary by Category

| Category  | Total  | ✅     | ⚠️     | ❌     | 🔄    | ⬆️    | Parity %  |
| --------- | ------ | ------ | ------ | ------ | ----- | ----- | --------- |
| CORE      | 26     | 13     | 7      | 6      | 0     | 0     | 50%       |
| DATA      | 12     | 5      | 1      | 6      | 0     | 0     | 42%       |
| INTG      | 8      | 1      | 1      | 6      | 0     | 0     | 13%       |
| SEC       | 6      | 3      | 3      | 0      | 0     | 0     | 50%       |
| API       | 18     | 14     | 1      | 3      | 0     | 0     | 78%       |
| PERF      | 4      | 0      | 1      | 3      | 0     | 0     | 0%        |
| OBS       | 2      | 1      | 0      | 0      | 0     | 1     | 50%       |
| PROC      | 1      | 0      | 0      | 1      | 0     | 0     | 0%        |
| **Total** | **78** | **42** | **14** | **21** | **0** | **1** | **53.8%** |

---

## Critical Path to Parity

The minimum feature set required for production parity:

1. **Query Modes (P0)**

   - ❌ Global mode
   - ❌ Mix mode

2. **Multi-Tenancy (P0)**

   - ⚠️ Tenant isolation
   - ❌ Tenant RAG manager

3. **Core Quality (P1)**

   - ⚠️ Entity deduplication
   - ⚠️ Description summarization
   - ❌ Keyword extraction
   - ⚠️ Reranking
   - ⚠️ Token budget

4. **LLM Providers (P1)**
   - ❌ Anthropic (critical for some users)

---

## Quick Wins (Low Effort, High Value)

| Feature            | Effort | Impact | Notes               |
| ------------------ | ------ | ------ | ------------------- |
| Keyword Extraction | 2 days | High   | Enables global mode |
| LLM Cache Complete | 2 days | Medium | Cost savings        |
| Document Scan API  | 2 days | Medium | User convenience    |
| Bypass Mode        | 1 day  | Low    | Easy to add         |
| Prompt-only Query  | 1 day  | Low    | Debug feature       |

---

## Target Advantages (Features Exceeding Source)

| Feature       | Description                   | Benefit              |
| ------------- | ----------------------------- | -------------------- |
| Metrics API   | Prometheus-compatible metrics | Better observability |
| Type Safety   | Rust's type system            | Fewer runtime errors |
| Performance   | Native compilation            | Lower latency        |
| Memory Safety | Rust guarantees               | No memory leaks      |

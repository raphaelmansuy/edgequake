# EdgeQuake Business Rules Registry

> Central registry of all business rules enforced by EdgeQuake.
> Use BRXXXX references in validation code for traceability.

**Version**: 1.0.0 | **Last Updated**: 2026-01-09

---

## Quick Reference Index

| Category                                                 | ID Range      | Count |
| -------------------------------------------------------- | ------------- | ----- |
| [Data Integrity Rules](#data-integrity-rules-br00xx)     | BR0001-BR0020 | 10    |
| [Query Processing Rules](#query-processing-rules-br01xx) | BR0101-BR0120 | 8     |
| [Multi-Tenancy Rules](#multi-tenancy-rules-br02xx)       | BR0201-BR0220 | 6     |
| [Cost Management Rules](#cost-management-rules-br03xx)   | BR0301-BR0320 | 4     |
| [Security Rules](#security-rules-br04xx)                 | BR0401-BR0420 | 5     |

---

## Data Integrity Rules (BR00XX)

### BR0001 - Document ID Uniqueness

| Attribute       | Value                                                                                               |
| --------------- | --------------------------------------------------------------------------------------------------- |
| **ID**          | BR0001                                                                                              |
| **Rule**        | Document IDs must be unique within a tenant's workspace                                             |
| **Module**      | edgequake-storage                                                                                   |
| **Validation**  | [adapters/postgres/kv.rs#insert](../edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs) |
| **Consequence** | Duplicate ID → Error, document rejected                                                             |
| **Related**     | FEAT0001, UC0001                                                                                    |

```rust
// WHY: Prevents data corruption and ensures consistent lineage tracking
// VALIDATION: Unique constraint on (tenant_id, workspace_id, doc_id)
```

### BR0002 - Chunk Overlap Constraint

| Attribute       | Value                                                                             |
| --------------- | --------------------------------------------------------------------------------- |
| **ID**          | BR0002                                                                            |
| **Rule**        | Chunk overlap must be less than chunk size                                        |
| **Module**      | edgequake-pipeline                                                                |
| **Validation**  | [chunker.rs#ChunkerConfig](../edgequake/crates/edgequake-pipeline/src/chunker.rs) |
| **Consequence** | Invalid config → Error at initialization                                          |
| **Related**     | FEAT0002, FEAT0301, FEAT0302                                                      |

```rust
// WHY: Overlap >= chunk_size would create infinite loops or empty chunks
// VALIDATION: assert!(overlap < chunk_size, "overlap must be < chunk_size")
```

### BR0003 - Entity Name Format

| Attribute       | Value                                                                                             |
| --------------- | ------------------------------------------------------------------------------------------------- |
| **ID**          | BR0003                                                                                            |
| **Rule**        | Entity names must be normalized to UPPERCASE_UNDERSCORED format                                   |
| **Module**      | edgequake-pipeline                                                                                |
| **Validation**  | [prompts/mod.rs#normalize_entity_name](../edgequake/crates/edgequake-pipeline/src/prompts/mod.rs) |
| **Consequence** | Non-normalized names → Auto-normalized before storage                                             |
| **Related**     | FEAT0003, FEAT0009                                                                                |

```rust
// WHY: Consistent naming enables reliable entity deduplication and graph merging
// TRANSFORMATION: "Sarah Chen" → "SARAH_CHEN", "machine-learning" → "MACHINE_LEARNING"
```

### BR0004 - Relationship Bidirectionality

| Attribute       | Value                                                             |
| --------------- | ----------------------------------------------------------------- |
| **ID**          | BR0004                                                            |
| **Rule**        | Relationships have a source and target; direction matters         |
| **Module**      | edgequake-pipeline                                                |
| **Validation**  | [merger.rs](../edgequake/crates/edgequake-pipeline/src/merger.rs) |
| **Consequence** | Same (source, target) with different descriptions → Merged        |
| **Related**     | FEAT0004, FEAT0005                                                |

```rust
// WHY: Graph semantics require directionality for meaningful traversal
// NOTE: (A→B) and (B→A) are stored as separate edges
```

### BR0005 - Embedding Dimension Match

| Attribute       | Value                                                                                                |
| --------------- | ---------------------------------------------------------------------------------------------------- |
| **ID**          | BR0005                                                                                               |
| **Rule**        | All embeddings must match configured dimension                                                       |
| **Module**      | edgequake-storage                                                                                    |
| **Validation**  | [adapters/postgres/vector.rs](../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs) |
| **Consequence** | Dimension mismatch → Error, embedding rejected                                                       |
| **Related**     | FEAT0006, FEAT0203                                                                                   |

```rust
// WHY: pgvector requires consistent dimensions for similarity search
// DEFAULT: 1536 dimensions (OpenAI text-embedding-3-small)
```

### BR0006 - Entity Description Length

| Attribute       | Value                                                                     |
| --------------- | ------------------------------------------------------------------------- |
| **ID**          | BR0006                                                                    |
| **Rule**        | Entity descriptions should not exceed 10,000 characters                   |
| **Module**      | edgequake-pipeline                                                        |
| **Validation**  | [summarizer.rs](../edgequake/crates/edgequake-pipeline/src/summarizer.rs) |
| **Consequence** | Long descriptions → Auto-summarized                                       |
| **Related**     | FEAT0010, FEAT0003                                                        |

```rust
// WHY: Very long descriptions waste tokens and reduce query quality
// ACTION: Trigger LLM summarization when description grows too long
```

### BR0007 - Lineage Immutability

| Attribute       | Value                                                               |
| --------------- | ------------------------------------------------------------------- |
| **ID**          | BR0007                                                              |
| **Rule**        | Lineage records are append-only; never modified                     |
| **Module**      | edgequake-pipeline                                                  |
| **Validation**  | [lineage.rs](../edgequake/crates/edgequake-pipeline/src/lineage.rs) |
| **Consequence** | Attempt to modify → Error                                           |
| **Related**     | FEAT0011                                                            |

```rust
// WHY: Lineage provides audit trail; modification would break traceability
// PATTERN: New extractions add new source spans, never overwrite
```

### BR0008 - Document Status Transitions

| Attribute       | Value                                                              |
| --------------- | ------------------------------------------------------------------ |
| **ID**          | BR0008                                                             |
| **Rule**        | Document status follows defined state machine                      |
| **Module**      | edgequake-core                                                     |
| **Validation**  | [types/document.rs](../edgequake/crates/edgequake-core/src/types/) |
| **Consequence** | Invalid transition → Error                                         |
| **Related**     | FEAT0001, FEAT0019                                                 |

```
State Machine:
┌──────────┐    ┌────────────┐    ┌───────────┐
│ Pending  │───▶│ Processing │───▶│ Completed │
└──────────┘    └─────┬──────┘    └───────────┘
                      │
                      ▼
               ┌───────────┐
               │  Failed   │
               └───────────┘
```

### BR0009 - Chunk Line Number Accuracy

| Attribute       | Value                                                                                      |
| --------------- | ------------------------------------------------------------------------------------------ |
| **ID**          | BR0009                                                                                     |
| **Rule**        | Chunk line numbers must accurately reflect source document                                 |
| **Module**      | edgequake-pipeline                                                                         |
| **Validation**  | [chunker.rs#calculate_line_numbers](../edgequake/crates/edgequake-pipeline/src/chunker.rs) |
| **Consequence** | Incorrect lines → Misleading source citations                                              |
| **Related**     | FEAT0002, FEAT0011                                                                         |

```rust
// WHY: Source citations depend on accurate line mapping
// INVARIANT: start_line <= end_line, lines match character positions
```

### BR0010 - Graph Cycle Detection

| Attribute       | Value                                                                                              |
| --------------- | -------------------------------------------------------------------------------------------------- |
| **ID**          | BR0010                                                                                             |
| **Rule**        | Self-referential entities are allowed; detected and marked                                         |
| **Module**      | edgequake-storage                                                                                  |
| **Validation**  | [adapters/postgres/graph.rs](../edgequake/crates/edgequake-storage/src/adapters/postgres/graph.rs) |
| **Consequence** | Self-loop → Stored but flagged for special handling                                                |
| **Related**     | FEAT0005, FEAT0204                                                                                 |

```rust
// WHY: Some domains have legitimate self-references (e.g., "recursion")
// HANDLING: Query engine handles cycles to prevent infinite traversal
```

---

## Query Processing Rules (BR01XX)

### BR0101 - Token Budget Limit

| Attribute       | Value                                                                     |
| --------------- | ------------------------------------------------------------------------- |
| **ID**          | BR0101                                                                    |
| **Rule**        | Total context tokens must not exceed LLM context window                   |
| **Module**      | edgequake-core                                                            |
| **Validation**  | [token_budget.rs](../edgequake/crates/edgequake-core/src/token_budget.rs) |
| **Consequence** | Over-budget → Context truncated                                           |
| **Related**     | FEAT0108, FEAT0007                                                        |

```rust
// WHY: LLMs have hard token limits (e.g., 128K for GPT-4)
// DEFAULT: 80% of model limit reserved for context
```

### BR0102 - Graph Context Priority

| Attribute       | Value                                                                                  |
| --------------- | -------------------------------------------------------------------------------------- |
| **ID**          | BR0102                                                                                 |
| **Rule**        | In hybrid modes, graph context takes priority over naive chunks                        |
| **Module**      | edgequake-query                                                                        |
| **Validation**  | [truncation.rs#balance_context](../edgequake/crates/edgequake-query/src/truncation.rs) |
| **Consequence** | Budget overflow → Naive chunks truncated first                                         |
| **Related**     | FEAT0104, FEAT0108                                                                     |

```rust
// WHY: Graph context is more semantically rich than raw text chunks
// ALLOCATION: entities (40%), relationships (30%), chunks (30%)
```

### BR0103 - Query Mode Validation

| Attribute       | Value                                                        |
| --------------- | ------------------------------------------------------------ |
| **ID**          | BR0103                                                       |
| **Rule**        | Query mode must be a valid enum value                        |
| **Module**      | edgequake-query                                              |
| **Validation**  | [modes.rs](../edgequake/crates/edgequake-query/src/modes.rs) |
| **Consequence** | Invalid mode → Error with valid options listed               |
| **Related**     | FEAT0007                                                     |

```rust
// VALID MODES: naive, local, global, hybrid, mix, bypass
// DEFAULT: hybrid
```

### BR0104 - Streaming Format

| Attribute       | Value                                                          |
| --------------- | -------------------------------------------------------------- |
| **ID**          | BR0104                                                         |
| **Rule**        | Streaming responses must use Server-Sent Events (SSE) format   |
| **Module**      | edgequake-api                                                  |
| **Validation**  | [streaming/](../edgequake/crates/edgequake-api/src/streaming/) |
| **Consequence** | Non-SSE response → Client parsing failure                      |
| **Related**     | FEAT0008, FEAT0404                                             |

```
SSE Format:
data: {"type": "content", "content": "chunk"}

data: {"type": "done", "sources": [...]}
```

### BR0105 - Empty Query Handling

| Attribute       | Value                                                                |
| --------------- | -------------------------------------------------------------------- |
| **ID**          | BR0105                                                               |
| **Rule**        | Empty or whitespace-only queries are rejected                        |
| **Module**      | edgequake-api                                                        |
| **Validation**  | [validation.rs](../edgequake/crates/edgequake-api/src/validation.rs) |
| **Consequence** | Empty query → 400 Bad Request                                        |
| **Related**     | UC0201, FEAT0403                                                     |

```rust
// WHY: LLM calls with empty prompts waste resources and return garbage
// VALIDATION: query.trim().is_empty() → reject
```

### BR0106 - Keyword Extraction Limit

| Attribute       | Value                                                                |
| --------------- | -------------------------------------------------------------------- |
| **ID**          | BR0106                                                               |
| **Rule**        | Maximum 20 keywords extracted per query                              |
| **Module**      | edgequake-query                                                      |
| **Validation**  | [keywords/mod.rs](../edgequake/crates/edgequake-query/src/keywords/) |
| **Consequence** | Excess keywords → Truncated to top 20 by relevance                   |
| **Related**     | FEAT0107                                                             |

```rust
// WHY: Too many keywords dilute search precision
// STRATEGY: Rank by tf-idf score, take top 20
```

### BR0107 - Conversation History Limit

| Attribute       | Value                                                          |
| --------------- | -------------------------------------------------------------- |
| **ID**          | BR0107                                                         |
| **Rule**        | Conversation context limited to last N messages                |
| **Module**      | edgequake-query                                                |
| **Validation**  | [engine.rs](../edgequake/crates/edgequake-query/src/engine.rs) |
| **Consequence** | Old messages → Excluded from context                           |
| **Related**     | FEAT0017, UC0204                                               |

```rust
// WHY: Very long conversations exceed token budget
// DEFAULT: Last 10 messages included
```

### BR0108 - Vector Search K Limit

| Attribute       | Value                                                                          |
| --------------- | ------------------------------------------------------------------------------ |
| **ID**          | BR0108                                                                         |
| **Rule**        | Vector search K (top results) limited to 100                                   |
| **Module**      | edgequake-storage                                                              |
| **Validation**  | [traits/vector.rs](../edgequake/crates/edgequake-storage/src/traits/vector.rs) |
| **Consequence** | K > 100 → Capped to 100                                                        |
| **Related**     | FEAT0101, FEAT0203                                                             |

```rust
// WHY: Performance degrades with very high K; rarely needed
// ENFORCEMENT: min(requested_k, 100)
```

---

## Multi-Tenancy Rules (BR02XX)

### BR0201 - Tenant Isolation

| Attribute       | Value                                                                         |
| --------------- | ----------------------------------------------------------------------------- |
| **ID**          | BR0201                                                                        |
| **Rule**        | All data operations must include tenant context                               |
| **Module**      | edgequake-core                                                                |
| **Validation**  | [tenant_manager.rs](../edgequake/crates/edgequake-core/src/tenant_manager.rs) |
| **Consequence** | Missing tenant → 401 Unauthorized                                             |
| **Related**     | FEAT0015, FEAT0701                                                            |

```rust
// WHY: Data leakage between tenants is a critical security violation
// ENFORCEMENT: All storage queries include tenant_id in WHERE clause
```

### BR0202 - API Key Mapping

| Attribute       | Value                                                                 |
| --------------- | --------------------------------------------------------------------- |
| **ID**          | BR0202                                                                |
| **Rule**        | Each API key maps to exactly one tenant                               |
| **Module**      | edgequake-auth                                                        |
| **Validation**  | [extractors.rs](../edgequake/crates/edgequake-auth/src/extractors.rs) |
| **Consequence** | Invalid key → 401 Unauthorized                                        |
| **Related**     | FEAT0701, BR0201                                                      |

```rust
// WHY: API keys provide tenant identification; one-to-many would be ambiguous
// LOOKUP: api_key → tenant_id (cached for performance)
```

### BR0203 - Cross-Tenant Query Forbidden

| Attribute       | Value                                                                 |
| --------------- | --------------------------------------------------------------------- |
| **ID**          | BR0203                                                                |
| **Rule**        | Queries cannot access data from other tenants                         |
| **Module**      | edgequake-storage                                                     |
| **Validation**  | All storage adapters                                                  |
| **Consequence** | Cross-tenant attempt → Empty result (no error to prevent enumeration) |
| **Related**     | FEAT0015, BR0201                                                      |

```rust
// WHY: Tenant isolation is non-negotiable for security
// IMPLEMENTATION: tenant_id hardcoded in query, not user-controllable
```

### BR0204 - Per-Tenant Rate Limits

| Attribute       | Value                                                                   |
| --------------- | ----------------------------------------------------------------------- |
| **ID**          | BR0204                                                                  |
| **Rule**        | Rate limits are applied per tenant, not globally                        |
| **Module**      | edgequake-rate-limiter                                                  |
| **Validation**  | [limiter.rs](../edgequake/crates/edgequake-rate-limiter/src/limiter.rs) |
| **Consequence** | Rate exceeded → 429 Too Many Requests                                   |
| **Related**     | FEAT0018                                                                |

```rust
// WHY: One tenant's load should not affect others
// IMPLEMENTATION: Separate token buckets per tenant
```

### BR0205 - Tenant Plan Enforcement

| Attribute       | Value                                                                         |
| --------------- | ----------------------------------------------------------------------------- |
| **ID**          | BR0205                                                                        |
| **Rule**        | Resource limits are enforced based on tenant plan                             |
| **Module**      | edgequake-core                                                                |
| **Validation**  | [tenant_manager.rs](../edgequake/crates/edgequake-core/src/tenant_manager.rs) |
| **Consequence** | Limit exceeded → 403 Forbidden with upgrade message                           |
| **Related**     | FEAT0015                                                                      |

```rust
// PLAN LIMITS:
// - free: 1000 documents, 100 queries/day
// - pro: 10000 documents, 1000 queries/day
// - enterprise: unlimited
```

### BR0206 - Workspace Ownership

| Attribute       | Value                                                                               |
| --------------- | ----------------------------------------------------------------------------------- |
| **ID**          | BR0206                                                                              |
| **Rule**        | Workspaces belong to exactly one tenant                                             |
| **Module**      | edgequake-core                                                                      |
| **Validation**  | [workspace_service.rs](../edgequake/crates/edgequake-core/src/workspace_service.rs) |
| **Consequence** | Orphan workspace → Cleaned up                                                       |
| **Related**     | FEAT0016, BR0201                                                                    |

```rust
// WHY: Orphan workspaces would be inaccessible and waste resources
// CONSTRAINT: workspace.tenant_id NOT NULL
```

---

## Cost Management Rules (BR03XX)

### BR0301 - LLM Call Tracking

| Attribute       | Value                                                                             |
| --------------- | --------------------------------------------------------------------------------- |
| **ID**          | BR0301                                                                            |
| **Rule**        | All LLM API calls must be tracked for billing                                     |
| **Module**      | edgequake-pipeline                                                                |
| **Validation**  | [progress.rs#CostTracker](../edgequake/crates/edgequake-pipeline/src/progress.rs) |
| **Consequence** | Untracked call → Audit violation                                                  |
| **Related**     | FEAT0013                                                                          |

```rust
// TRACKED METRICS:
// - prompt_tokens: tokens sent to LLM
// - completion_tokens: tokens received from LLM
// - model: model name for pricing lookup
// - timestamp: for time-based billing
```

### BR0302 - Cache Before Call

| Attribute       | Value                                                      |
| --------------- | ---------------------------------------------------------- |
| **ID**          | BR0302                                                     |
| **Rule**        | Check cache before making LLM API call                     |
| **Module**      | edgequake-llm                                              |
| **Validation**  | [cache.rs](../edgequake/crates/edgequake-llm/src/cache.rs) |
| **Consequence** | Cache hit → Return cached, no API call                     |
| **Related**     | FEAT0014, BR0301                                           |

```rust
// WHY: LLM calls are expensive; caching saves 90%+ for repeated extractions
// CACHE KEY: hash(prompt + model + temperature)
```

### BR0303 - Batch Processing

| Attribute       | Value                                                                 |
| --------------- | --------------------------------------------------------------------- |
| **ID**          | BR0303                                                                |
| **Rule**        | Prefer batch processing over individual LLM calls                     |
| **Module**      | edgequake-pipeline                                                    |
| **Validation**  | [pipeline.rs](../edgequake/crates/edgequake-pipeline/src/pipeline.rs) |
| **Consequence** | Individual calls → Higher latency and cost                            |
| **Related**     | FEAT0001, BR0301                                                      |

```rust
// WHY: Batch calls have lower per-token cost with some providers
// DEFAULT BATCH SIZE: 5 chunks per extraction call
```

### BR0304 - Cost Estimation Display

| Attribute       | Value                                                                 |
| --------------- | --------------------------------------------------------------------- |
| **ID**          | BR0304                                                                |
| **Rule**        | Display cost estimates before expensive operations                    |
| **Module**      | edgequake-pipeline                                                    |
| **Validation**  | [progress.rs](../edgequake/crates/edgequake-pipeline/src/progress.rs) |
| **Consequence** | Large document → Cost warning in progress                             |
| **Related**     | FEAT0012, FEAT0013                                                    |

```rust
// WHY: Users should make informed decisions about processing costs
// THRESHOLD: Warn if estimated cost > $1.00
```

---

## Security Rules (BR04XX)

### BR0401 - Input Sanitization

| Attribute       | Value                                                                       |
| --------------- | --------------------------------------------------------------------------- |
| **ID**          | BR0401                                                                      |
| **Rule**        | All user input must be sanitized before LLM prompts                         |
| **Module**      | edgequake-pipeline                                                          |
| **Validation**  | [prompts/mod.rs](../edgequake/crates/edgequake-pipeline/src/prompts/mod.rs) |
| **Consequence** | Prompt injection → Mitigated by sanitization                                |
| **Related**     | FEAT0003, FEAT0007                                                          |

```rust
// WHY: Prevent prompt injection attacks
// SANITIZATION: Escape control characters, limit input length
```

### BR0402 - File Type Validation

| Attribute       | Value                                                                          |
| --------------- | ------------------------------------------------------------------------------ |
| **ID**          | BR0402                                                                         |
| **Rule**        | Only allowed file types can be uploaded                                        |
| **Module**      | edgequake-api                                                                  |
| **Validation**  | [file_validation.rs](../edgequake/crates/edgequake-api/src/file_validation.rs) |
| **Consequence** | Invalid type → 415 Unsupported Media Type                                      |
| **Related**     | FEAT0402                                                                       |

```rust
// ALLOWED: .pdf, .txt, .md, .json
// VALIDATION: Check MIME type AND extension
```

### BR0403 - File Size Limit

| Attribute       | Value                                                                          |
| --------------- | ------------------------------------------------------------------------------ |
| **ID**          | BR0403                                                                         |
| **Rule**        | Uploaded files must not exceed size limit                                      |
| **Module**      | edgequake-api                                                                  |
| **Validation**  | [file_validation.rs](../edgequake/crates/edgequake-api/src/file_validation.rs) |
| **Consequence** | Oversized → 413 Payload Too Large                                              |
| **Related**     | FEAT0402                                                                       |

```rust
// DEFAULT LIMIT: 100MB
// CONFIGURABLE: via MAX_UPLOAD_SIZE env var
```

### BR0404 - Password Hashing

| Attribute       | Value                                                             |
| --------------- | ----------------------------------------------------------------- |
| **ID**          | BR0404                                                            |
| **Rule**        | Passwords must be hashed using Argon2                             |
| **Module**      | edgequake-auth                                                    |
| **Validation**  | [password.rs](../edgequake/crates/edgequake-auth/src/password.rs) |
| **Consequence** | Plain password stored → Critical security violation               |
| **Related**     | FEAT0702                                                          |

```rust
// WHY: Industry standard for password hashing
// ALGORITHM: Argon2id with memory=64MB, iterations=3
```

### BR0405 - Audit Log Retention

| Attribute       | Value                                                          |
| --------------- | -------------------------------------------------------------- |
| **ID**          | BR0405                                                         |
| **Rule**        | Audit logs must be retained for 90 days minimum                |
| **Module**      | edgequake-audit                                                |
| **Validation**  | [logger.rs](../edgequake/crates/edgequake-audit/src/logger.rs) |
| **Consequence** | Early deletion → Compliance violation                          |
| **Related**     | FEAT0020                                                       |

```rust
// WHY: Required for security investigations and compliance
// STORAGE: Separate audit log table with TTL
```

---

## PDF Processing Rules (BR10XX)

> Business rules specific to PDF extraction and conversion quality.

### BR1001 - Preserve Document Structure

| Attribute       | Value                                                                              |
| --------------- | ---------------------------------------------------------------------------------- |
| **ID**          | BR1001                                                                             |
| **Rule**        | Document structure (headings, lists, paragraphs) must be preserved in output       |
| **Module**      | edgequake-pdf                                                                      |
| **Validation**  | [processors/](../edgequake/crates/edgequake-pdf/src/processors/)                   |
| **Consequence** | Lost structure → Degraded RAG retrieval quality                                    |
| **Related**     | FEAT1001, FEAT1022                                                                 |

```rust
// WHY: Structure preservation enables accurate semantic chunking
// MEASURE: Heading count in output >= 80% of visual headings
```

### BR1002 - Graceful Malformed PDF Handling

| Attribute       | Value                                                                              |
| --------------- | ---------------------------------------------------------------------------------- |
| **ID**          | BR1002                                                                             |
| **Rule**        | Malformed PDFs must be handled gracefully without crashing                         |
| **Module**      | edgequake-pdf                                                                      |
| **Validation**  | [extractor.rs](../edgequake/crates/edgequake-pdf/src/extractor.rs)                 |
| **Consequence** | Crash on malformed PDF → User data loss, service outage                            |
| **Related**     | FEAT1001, BR1003                                                                   |

```rust
// WHY: Real-world PDFs are often malformed or non-compliant
// STRATEGY: Fall back to basic text extraction, log warning
```

### BR1003 - Reading Order Accuracy

| Attribute       | Value                                                                              |
| --------------- | ---------------------------------------------------------------------------------- |
| **ID**          | BR1003                                                                             |
| **Rule**        | Reading order accuracy must exceed 95% for single-column documents                 |
| **Module**      | edgequake-pdf                                                                      |
| **Validation**  | [layout/](../edgequake/crates/edgequake-pdf/src/layout/)                           |
| **Consequence** | Incorrect order → Garbled context, unusable for RAG                                |
| **Related**     | FEAT1003, FEAT1001                                                                 |

```rust
// WHY: Incorrect reading order destroys semantic meaning
// MEASURE: Levenshtein similarity with gold standard > 0.95
```

### BR1004 - Table Cell Alignment

| Attribute       | Value                                                                              |
| --------------- | ---------------------------------------------------------------------------------- |
| **ID**          | BR1004                                                                             |
| **Rule**        | Table cell alignment must be preserved in Markdown output                          |
| **Module**      | edgequake-pdf                                                                      |
| **Validation**  | [backend/lattice.rs](../edgequake/crates/edgequake-pdf/src/backend/lattice.rs)     |
| **Consequence** | Misaligned cells → Incorrect data associations                                     |
| **Related**     | FEAT1002, FEAT0503                                                                 |

```rust
// WHY: Tables encode relationships between row/column headers and values
// MEASURE: Cell count in output matches visual cell count ±10%
```

### BR1010 - Font Size Threshold for Headings

| Attribute       | Value                                                                              |
| --------------- | ---------------------------------------------------------------------------------- |
| **ID**          | BR1010                                                                             |
| **Rule**        | Font size must be ≥20% larger than body text for heading classification           |
| **Module**      | edgequake-pdf                                                                      |
| **Validation**  | [processors/structure_detection.rs](../edgequake/crates/edgequake-pdf/src/processors/structure_detection.rs) |
| **Consequence** | Too low threshold → False positive headings                                        |
| **Related**     | FEAT0505, FEAT1010                                                                 |

```rust
// WHY: 20% threshold reduces false positives from emphasis/captions
// CONFIG: heading_size_ratio = 1.2 (configurable)
```

### BR1011 - Maximum Heading Length

| Attribute       | Value                                                                              |
| --------------- | ---------------------------------------------------------------------------------- |
| **ID**          | BR1011                                                                             |
| **Rule**        | Headings must be ≤200 characters; longer text is paragraph content                 |
| **Module**      | edgequake-pdf                                                                      |
| **Validation**  | [processors/structure_detection.rs](../edgequake/crates/edgequake-pdf/src/processors/structure_detection.rs) |
| **Consequence** | No length limit → Entire paragraphs classified as headings                         |
| **Related**     | FEAT0505, BR1010                                                                   |

```rust
// WHY: Headings are short; long "headings" are typically styled paragraphs
// CONFIG: max_heading_length = 200
```

### BR1020 - Processor Chain Order

| Attribute       | Value                                                                              |
| --------------- | ---------------------------------------------------------------------------------- |
| **ID**          | BR1020                                                                             |
| **Rule**        | Processors must execute in deterministic order for reproducible output             |
| **Module**      | edgequake-pdf                                                                      |
| **Validation**  | [processors/processor.rs](../edgequake/crates/edgequake-pdf/src/processors/processor.rs) |
| **Consequence** | Non-deterministic order → Inconsistent extraction results                          |
| **Related**     | FEAT1020, BR1001                                                                   |

```rust
// CHAIN ORDER:
// 1. LayoutProcessor (reading order)
// 2. TableDetectionProcessor
// 3. HeaderDetectionProcessor
// 4. PostProcessor (cleanup)
```

### BR1021 - Deduplication by Bounding Box

| Attribute       | Value                                                                              |
| --------------- | ---------------------------------------------------------------------------------- |
| **ID**          | BR1021                                                                             |
| **Rule**        | Overlapping text elements with same content must be deduplicated                   |
| **Module**      | edgequake-pdf                                                                      |
| **Validation**  | [backend/sota_backend.rs](../edgequake/crates/edgequake-pdf/src/backend/sota_backend.rs) |
| **Consequence** | Duplicate text → Garbled output, inflated token count                              |
| **Related**     | FEAT1001, FEAT1021                                                                 |

```rust
// WHY: PDF rendering sometimes overlays text for visual effects
// ALGORITHM: Merge if bbox overlap > 80% and text identical
```

### BR1023 - Skip Corrupt Images

| Attribute       | Value                                                                              |
| --------------- | ---------------------------------------------------------------------------------- |
| **ID**          | BR1023                                                                             |
| **Rule**        | Corrupt or unsupported image formats must be skipped, not crash extraction         |
| **Module**      | edgequake-pdf                                                                      |
| **Validation**  | [image_extraction.rs](../edgequake/crates/edgequake-pdf/src/image_extraction.rs)   |
| **Consequence** | Crash on corrupt image → Full document extraction fails                            |
| **Related**     | FEAT1004, FEAT1023                                                                 |

```rust
// WHY: One corrupt image should not prevent text extraction
// ACTION: Log warning, continue with text content
```

### BR1024 - Image Size Limit

| Attribute       | Value                                                                              |
| --------------- | ---------------------------------------------------------------------------------- |
| **ID**          | BR1024                                                                             |
| **Rule**        | Extracted images must be limited to 10MB to prevent memory exhaustion              |
| **Module**      | edgequake-pdf                                                                      |
| **Validation**  | [image_extraction.rs](../edgequake/crates/edgequake-pdf/src/image_extraction.rs)   |
| **Consequence** | Unlimited size → OOM on high-resolution images                                     |
| **Related**     | FEAT1004, BR1023                                                                   |

```rust
// WHY: Vision API limits + memory safety
// CONFIG: max_image_size_bytes = 10 * 1024 * 1024
```

### BR1025 - OCR Language Detection

| Attribute       | Value                                                                              |
| --------------- | ---------------------------------------------------------------------------------- |
| **ID**          | BR1025                                                                             |
| **Rule**        | OCR should auto-detect language when not specified                                 |
| **Module**      | edgequake-pdf                                                                      |
| **Validation**  | [image_ocr.rs](../edgequake/crates/edgequake-pdf/src/image_ocr.rs)                 |
| **Consequence** | Wrong language → Poor OCR accuracy                                                 |
| **Related**     | FEAT1024, FEAT1004                                                                 |

```rust
// WHY: LLM-based OCR handles multi-language automatically
// FALLBACK: Assume English if detection fails
```

### BR1026 - Vision API Rate Limiting

| Attribute       | Value                                                                              |
| --------------- | ---------------------------------------------------------------------------------- |
| **ID**          | BR1026                                                                             |
| **Rule**        | Vision API calls must respect rate limits (max 10 requests/minute default)         |
| **Module**      | edgequake-pdf                                                                      |
| **Validation**  | [vision.rs](../edgequake/crates/edgequake-pdf/src/vision.rs)                       |
| **Consequence** | Rate limit exceeded → API errors, document processing failure                      |
| **Related**     | FEAT1024, BR0301                                                                   |

```rust
// WHY: Vision APIs have stricter rate limits than text APIs
// CONFIG: vision_rate_limit_rpm = 10
```

---

## Summary Statistics

| Category         | Total  | Critical | High   | Medium |
| ---------------- | ------ | -------- | ------ | ------ |
| Data Integrity   | 10     | 4        | 4      | 2      |
| Query Processing | 8      | 2        | 3      | 3      |
| Multi-Tenancy    | 6      | 4        | 2      | 0      |
| Cost Management  | 4      | 0        | 2      | 2      |
| Security         | 5      | 3        | 2      | 0      |
| PDF Processing   | 12     | 2        | 6      | 4      |
| **TOTAL**        | **45** | **15**   | **19** | **11** |

---

## Related Documents

- [Features Registry](features.md)
- [Use Cases](use_cases.md)
- [Configuration Reference](0007-configuration-reference.md)
- [Security Guide](0006-deployment-guide.md#security)

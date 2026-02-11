# Iteration 10 — Act

## Changes Made

### Files Modified (7 files, ~400 insertions, ~150 deletions)

#### Type Rewrites

1. **src/types/documents.ts** — Complete rewrite:
   - Added `DocumentCostInfo` (7 fields: total_cost_usd, formatted_cost, tokens, models)
   - Added `StatusCounts` (6 status counters)
   - Added `ListDocumentsResponse` (7 fields: documents, pagination, status_counts)
   - Added `DocumentSummary` (25+ fields matching Rust DocumentSummary)
   - Fixed `UploadDocumentResponse` — added task_id, duplicate_of, chunk_count, entity_count, relationship_count, cost
   - Changed `ListDocumentsQuery` from limit/offset to page/page_size
   - Updated `DocumentDetail` to standalone type
   - Legacy alias: `DocumentInfo = DocumentSummary`

2. **src/types/query.ts** — Major rewrite:
   - Renamed `QuerySource` → `SourceReference` (11 fields matching Rust: source_type, id, score, rerank_score, snippet, reference_id, document_id, file_path, start_line, end_line, chunk_index)
   - Added `QueryStats` (10 fields: timing metrics, token counts, model lineage)
   - Fixed `QueryResponse` — added stats, conversation_id, reranked; removed made-up context/prompt fields
   - Deprecated alias: `QuerySource = SourceReference`

3. **src/types/chat.ts** — Significant rewrite:
   - Removed local `SourceReference` and `QueryStats` — now imports from query.ts
   - Fixed `ChatCompletionRequest` — added max_tokens, temperature, top_k, parent_id; renamed llm_provider→provider, llm_model→model; removed non-existent conversation_history/enable_rerank
   - Re-exports shared types for backward compatibility

4. **src/types/graph.ts** — Graph type polish:
   - Added `node_type?`, `description?` to `GraphNode`
   - Changed `GraphEdge.label` → `edge_type`; removed `id`
   - Added `is_truncated?` to `GraphResponse`
   - Fixed `SearchNodesResponse` — added edges, total_matches, is_truncated
   - Fixed `SearchLabelsResponse` — labels as string[] (matches Rust)
   - Fixed `PopularLabelsResponse` — full label metadata (label, entity_type, degree, description)

#### Resource Updates

5. **src/resources/documents.ts** — Removed Paginator, returns `Promise<ListDocumentsResponse>` with page/page_size params

#### Test Updates

6. **tests/unit/resources.test.ts** — Updated documents list mock (structured response with status_counts)
7. **tests/e2e/documents.test.ts** — Updated list() usage from Paginator to direct response

### Validation Results

| Metric     | Value                    |
| ---------- | ------------------------ |
| Unit tests | 247 passed               |
| E2E tests  | 62 passed (8 test files) |
| Coverage   | 98.12% statements        |
| Type check | Clean (0 errors)         |
| Build ESM  | 48.25 KB                 |
| Build CJS  | 48.78 KB                 |
| Build DTS  | 91.21 KB (was 83.97 KB)  |

### Type Interface Summary (After Iteration 10)

| Module           | Interfaces | Key Changes                                                                |
| ---------------- | ---------- | -------------------------------------------------------------------------- |
| documents.ts     | 11         | +DocumentCostInfo, +StatusCounts, +ListDocumentsResponse, +DocumentSummary |
| query.ts         | 7          | +SourceReference, +QueryStats; QueryResponse expanded                      |
| chat.ts          | 4          | ChatCompletionRequest fixed; shared type imports                           |
| graph.ts         | 22         | GraphNode/Edge/Response updated; SearchNodesResponse fixed                 |
| costs.ts         | 14         | (unchanged from IMPL-09)                                                   |
| lineage.ts       | 20         | (unchanged from IMPL-09)                                                   |
| conversations.ts | 15         | (unchanged from IMPL-08)                                                   |
| auth.ts          | 10         | (unchanged from IMPL-08)                                                   |
| **Total**        | **~103**   | Phase 1 TypeScript SDK complete                                            |

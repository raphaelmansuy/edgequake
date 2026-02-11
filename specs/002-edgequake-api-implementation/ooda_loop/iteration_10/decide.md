# Iteration 10 — Decide

## Plan

### Phase A: Document Type Accuracy

1. Read `documents_types.rs` (full 1234 lines)
2. Rewrite `src/types/documents.ts` — add `DocumentCostInfo`, `StatusCounts`, `ListDocumentsResponse`, `DocumentSummary`; fix field names
3. Update documents resource — remove `Paginator`, return `Promise<ListDocumentsResponse>` directly
4. Update unit and E2E tests

### Phase B: Query/Chat Type Accuracy

5. Read `query_types.rs` and `chat_types.rs`
6. Rewrite `src/types/query.ts` — add `SourceReference` (replacing `QuerySource`), `QueryStats`, fix `QueryResponse`
7. Rewrite `src/types/chat.ts` — import shared types from query, fix `ChatCompletionRequest` fields
8. Verify backward compatibility with deprecation aliases

### Phase C: Graph Type Polish

9. Read `graph_types.rs`
10. Update `src/types/graph.ts` — add `node_type`, `description` to `GraphNode`; fix `GraphEdge` to use `edge_type`; add `is_truncated` to responses

### Phase D: Validation

11. Type check, unit tests (247), E2E tests (62)
12. Coverage report (target >90%)
13. Build (ESM + CJS + DTS)

### Phase E: Commit

14. Create OODA docs
15. Git commit as IMPL-10

# OODA-18: Orient — Query Engine Analysis

## Analysis

### Coverage Gap
Before OODA-18, there were zero E2E tests for the query engine — the primary user-facing
feature of EdgeQuake. The query API is complex with 5 modes, context-only, prompt-only,
reranking, and conversation history support.

### Design Decisions

1. **Two query modes tested (naive + hybrid)**: Rather than testing all 5 modes in one test
   (which would be slow), we test the two most common modes. The engine internally handles
   mode dispatch the same way for all modes.

2. **Conversation headers**: Conversation endpoints strictly require UUID tenant/user headers.
   We use dedicated constants (TEST_TENANT_ID, TEST_USER_ID, TEST_WORKSPACE_ID) separate
   from other test files to ensure isolation.

3. **Context-only assertion**: We verify answer is empty string (not null) because the
   sota_engine explicitly returns `(String::new(), 0)` for context_only.

4. **Sources validation**: With mock storage, sources may be empty. We validate array
   existence and field structure when sources are present.

### Risk Assessment
- **Low risk**: Query tests use in-memory state with mock provider
- **Mock behavior**: "Mock response" answer, empty context, 0 sources in naive mode
- **No side effects**: Each test creates isolated state

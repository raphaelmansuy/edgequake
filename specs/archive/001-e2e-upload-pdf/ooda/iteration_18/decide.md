# OODA-18: Decide — Query Engine Test Plan

## Decision

Create 11 E2E tests covering:

1. **test_basic_query_response_structure** — Validate answer, mode, sources, stats fields
2. **test_query_modes_naive_and_hybrid** — Two common modes return correct mode in response
3. **test_context_only_query** — Verify empty answer + valid structure
4. **test_prompt_only_query** — Verify formatted prompt returned
5. **test_create_conversation** — With tenant+user headers → 201
6. **test_create_conversation_no_headers** — Without headers → 400
7. **test_list_conversations** — Paginated response with items + pagination
8. **test_query_empty_knowledge_base** — 200 even with no documents
9. **test_query_empty_string** — Whitespace query → 422
10. **test_query_with_conversation_history** — Multi-turn context via conversation_history field
11. **test_query_response_sources** — Sources array present, validated when non-empty

## Rationale

- Query engine is the primary user-facing feature
- Tests cover happy path, validation, and conversation integration
- Conversation header requirements discovered and tested
- All tests use 30s timeout for query (which calls embedding + retrieval + generation)

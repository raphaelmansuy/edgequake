# Task Log: 2024-12-27 Client-Side Implementation Specs

## Actions

- Reviewed existing plan documents (01-06) in plan_improve_query_page/
- Analyzed EdgeQuake WebUI source code (query/, stores/, lib/api/)
- Studied open-webui markdown implementation patterns (Markdown.svelte, MarkdownTokens.svelte)
- Created 4 new client-side implementation specification documents

## Decisions

- Adopt marked.js lexer over react-markdown for token-based streaming
- Use `done` prop pattern from open-webui instead of `isStreaming` with fallback
- Separate UI state (Zustand) from server state (React Query)
- Use @tanstack/react-virtual for virtualized conversation list

## Documents Created

| Document                       | Lines | Purpose                                     |
| ------------------------------ | ----- | ------------------------------------------- |
| 07_client_markdown_pipeline.md | ~350  | Token-based markdown rendering architecture |
| 08_client_api_client.md        | ~400  | TypeScript API client and React Query hooks |
| 09_client_state_management.md  | ~380  | Zustand + React Query state architecture    |
| 10_client_history_panel.md     | ~400  | Virtualized history panel components        |

## Next Steps

1. Sprint 1 Week 1: Install marked.js and create token components
2. Sprint 1 Week 2: Implement streaming markdown without fallback
3. Sprint 2: Run database migration and implement API handlers
4. Sprint 2: Create React Query hooks and migrate from localStorage

## Lessons/Insights

- open-webui uses token-by-token rendering which avoids streaming fallback issues
- EdgeQuake's current normalizeMarkdown() with 60+ regexes indicates tokenization problems
- Separation of UI state from server state simplifies optimistic updates

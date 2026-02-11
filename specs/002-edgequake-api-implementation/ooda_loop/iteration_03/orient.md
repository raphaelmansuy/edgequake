# Iteration 03 — Orient

## Approach: Examples + CI/CD + Package Polish

### Examples Strategy

Create 8 standalone TypeScript example files that demonstrate real-world usage:

1. `basic_usage.ts` — Setup, health check, simple query
2. `document_upload.ts` — Text + PDF upload, track processing
3. `query_demo.ts` — Simple, advanced, and hybrid queries
4. `graph_exploration.ts` — Entity search, neighborhood traversal
5. `streaming_query.ts` — SSE streaming query + chat
6. `websocket_progress.ts` — WebSocket pipeline monitoring
7. `multi_tenant.ts` — Tenant/workspace management
8. `batch_operations.ts` — Bulk uploads, bulk delete, pagination

Each example must:

- Be self-contained (just `import { EdgeQuake } from "@edgequake/sdk"`)
- Include `// WHY:` comments for learning
- Handle errors gracefully
- Use `async/await` throughout

### CI/CD Strategy

- `.github/workflows/test.yml` — Run on PR, test + lint + typecheck
- `.github/workflows/publish.yml` — Run on version tag, publish to npm

### Package Polish

- Update package.json: exports, files, engines, scripts
- Create tsup.config.ts (already exists but verify)
- Add LICENSE (Apache 2.0)
- Add CHANGELOG.md

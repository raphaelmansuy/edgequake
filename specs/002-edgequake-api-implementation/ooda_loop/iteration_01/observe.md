# Iteration 01 — Observe

## Mission Re-Read ✓

Read `specs/002-edgequake-api-implementation.md` (601 lines). Phase 1: TypeScript SDK Foundation.

## API Surface Analysis

**Source**: `edgequake/crates/edgequake-api/src/routes.rs` (467 lines)

### Endpoint Count by Category

| Category      | Endpoints | Routes Source Lines |
| ------------- | --------- | ------------------- |
| Health        | 3         | L97-100             |
| Metrics       | 1         | L102                |
| WebSocket     | 2         | L103-109            |
| Ollama Compat | 5         | L114-120            |
| Auth          | 4         | L124-127            |
| Users         | 4         | L129-132            |
| API Keys      | 3         | L134-136            |
| Tenants       | 5         | L138-142            |
| Workspaces    | 12        | L144-195            |
| Documents     | 23        | L197-277            |
| Query         | 2         | L279-280            |
| Chat          | 2         | L282-286            |
| Conversations | 13        | L288-324            |
| Messages      | 2         | L326-327            |
| Folders       | 4         | L329-332            |
| Shared        | 1         | L334                |
| Graph         | 7         | L336-343            |
| Entities      | 8         | L345-364            |
| Relationships | 5         | L366-380            |
| Tasks         | 4         | L382-385            |
| Pipeline      | 3         | L387-391            |
| Costs         | 6         | L393-400            |
| Lineage       | 2         | L402-407            |
| Chunks        | 1         | L409                |
| Provenance    | 1         | L411-413            |
| Settings      | 2         | L415-421            |
| Models        | 6         | L423-428            |
| **Total**     | **~131**  |                     |

### TypeScript Design Spec Files (13 docs)

Located in `specs/api_design/typescript/`:

- 01-overview.md: Package structure, dependencies (zod, eventsource-parser)
- 02-architecture.md: Class hierarchy, transport, middleware, pagination, SSE
- 03-authentication.md: 4 auth methods, token refresh flow
- 04-client-design.md: EdgeQuake class, 21 resource namespaces
- 05-resource-apis.md: All 131 endpoint method signatures
- 07-error-handling.md: Error hierarchy (12 error classes)
- 09-streaming.md: SSE parsing, WebSocket wrapper

### Key Design Decisions Pre-Made

1. **No axios** — uses native `fetch()` (Node 18+, browser, Deno, Bun)
2. **Dependencies**: `zod ^3.23`, `eventsource-parser ^3.0` only
3. **Package name**: `@edgequake/sdk`
4. **Exports**: ESM primary, CJS fallback
5. **Ships .d.ts** declaration files

### Handler Type Files Analyzed

- `health_types.rs` (327 lines): HealthResponse, ComponentHealth, SchemaHealth, ProvidersHealth
- `auth_types.rs` (378 lines): LoginRequest/Response, JWT tokens, UserInfo, API keys
- `documents_types.rs` (1234 lines): Upload, List, Track, PDF, Scan, Reprocess
- `query_types.rs` (370 lines): QueryRequest, StreamQueryRequest, QueryResponse

## Existing SDK State

- No `sdks/` directory exists yet — this is a greenfield implementation
- Design specs are comprehensive and detailed
- `specs/api_design/00-api-contract.md` provides the shared contract

# Documentation Sync - Working Notes

**Status**: Phase 0: Code Discovery | **Last Updated**: 2025-12-28

## Recent E2E Testing Notes (2025-12-28)

### Query Page Validation Complete

Successfully tested the EdgeQuake WebUI Query page with Playwright browser automation:

**Bug Fixes Applied:**

1. **API Client Authentication** (`src/lib/api/client.ts`):

   - Added anonymous user ID generation and persistence
   - Fixed: Conversation APIs require X-User-ID header

2. **Folders API Response** (`src/lib/api/folders.ts`):
   - Fixed response type mismatch (array vs object wrapper)

**Verified Working:**

- Conversation creation via "New" button
- Message sending with Enter key
- Streaming response display
- History panel conversation list
- Switching between conversations
- Query mode selector (Local/Global/Hybrid/Simple)
- Markdown rendering with token-based approach

**Architecture Confirmed:**

- Backend: Rust Axum API on port 8080 with InMemory storage
- Frontend: Next.js 16.1.0 with React Query + Zustand
- Markdown: StreamingMarkdownRenderer using marked.lexer()

---

## MANDATORY COUNTS (For Verification)

- Total API Endpoints to Verify: 62
- Total Config Fields to Verify: 38
- Total Types to Verify: 6
- Total Storage Adapters to Verify: 2
- Total Examples to Verify: 5
- **REQUIRED FINDINGS LOG ENTRIES: 113**

## 0. Ground Truth Catalog (From Code)

| Category | Feature                                         | Source (File:Line)              | Documented? | Doc Location                         |
| :------- | :---------------------------------------------- | :------------------------------ | :---------- | :----------------------------------- |
| Endpoint | `GET /health`                                   | routes.rs:15                    | ✅          | 0003-api-reference.md:L84            |
| Endpoint | `GET /ready`                                    | routes.rs:16                    | ✅          | 0003-api-reference.md:L106           |
| Endpoint | `GET /live`                                     | routes.rs:17                    | ✅          | 0003-api-reference.md:L122           |
| Endpoint | `GET /metrics`                                  | routes.rs:19                    | ✅          | 0003-api-reference.md:L138           |
| Endpoint | `GET /version`                                  | routes.rs:33                    | ✅          | 0003-api-reference.md:L173           |
| Endpoint | `GET /tags`                                     | routes.rs:34                    | ✅          | 0003-api-reference.md:L189           |
| Endpoint | `GET /ps`                                       | routes.rs:35                    | ✅          | 0003-api-reference.md:L220           |
| Endpoint | `POST /generate`                                | routes.rs:36                    | ✅          | 0003-api-reference.md:L244           |
| Endpoint | `POST /chat`                                    | routes.rs:37                    | ✅          | 0003-api-reference.md:L272           |
| Endpoint | `POST /auth/login`                              | routes.rs:44                    | ✅          | 0003-api-reference.md:L303           |
| Endpoint | `POST /auth/refresh`                            | routes.rs:45                    | ✅          | 0003-api-reference.md:L337           |
| Endpoint | `POST /auth/logout`                             | routes.rs:46                    | ✅          | 0003-api-reference.md:L354           |
| Endpoint | `GET /auth/me`                                  | routes.rs:47                    | ✅          | 0003-api-reference.md:L363           |
| Endpoint | `POST /users`                                   | routes.rs:49                    | ✅          | 0003-api-reference.md:L378           |
| Endpoint | `GET /users`                                    | routes.rs:50                    | ✅          | 0003-api-reference.md:L419           |
| Endpoint | `GET /users/{user_id}`                          | routes.rs:51                    | ✅          | 0003-api-reference.md:L455           |
| Endpoint | `DELETE /users/{user_id}`                       | routes.rs:52                    | ✅          | 0003-api-reference.md:L464           |
| Endpoint | `POST /api-keys`                                | routes.rs:54                    | ✅          | 0003-api-reference.md:L479           |
| Endpoint | `GET /api-keys`                                 | routes.rs:55                    | ✅          | 0003-api-reference.md:L512           |
| Endpoint | `DELETE /api-keys/{key_id}`                     | routes.rs:56                    | ✅          | 0003-api-reference.md:L538           |
| Endpoint | `POST /tenants`                                 | routes.rs:58                    | ✅          | 0003-api-reference.md:L553           |
| Endpoint | `GET /tenants`                                  | routes.rs:59                    | ✅          | 0003-api-reference.md:L596           |
| Endpoint | `GET /tenants/{tenant_id}`                      | routes.rs:60                    | ✅          | 0003-api-reference.md:L616           |
| Endpoint | `PUT /tenants/{tenant_id}`                      | routes.rs:61                    | ✅          | 0003-api-reference.md:L625           |
| Endpoint | `DELETE /tenants/{tenant_id}`                   | routes.rs:62                    | ✅          | 0003-api-reference.md:L646           |
| Endpoint | `POST /workspaces`                              | routes.rs:64                    | ✅          | 0003-api-reference.md:L661           |
| Endpoint | `GET /workspaces`                               | routes.rs:68                    | ✅          | 0003-api-reference.md:L705           |
| Endpoint | `GET /workspaces/{workspace_id}`                | routes.rs:72                    | ✅          | 0003-api-reference.md:L714           |
| Endpoint | `PUT /workspaces/{workspace_id}`                | routes.rs:73                    | ✅          | 0003-api-reference.md:L723           |
| Endpoint | `DELETE /workspaces/{workspace_id}`             | routes.rs:77                    | ✅          | 0003-api-reference.md:L744           |
| Endpoint | `GET /workspaces/{workspace_id}/stats`          | routes.rs:81                    | ✅          | 0003-api-reference.md:L753           |
| Endpoint | `POST /documents`                               | routes.rs:86                    | ✅          | 0003-api-reference.md:L780           |
| Endpoint | `GET /documents`                                | routes.rs:87                    | ✅          | 0003-api-reference.md:L880           |
| Endpoint | `DELETE /documents/{document_id}`               | routes.rs:89                    | ✅          | 0003-api-reference.md:L938           |
| Endpoint | `POST /documents/upload`                        | routes.rs:94                    | ✅          | 0003-api-reference.md:L837           |
| Endpoint | `POST /documents/batch`                         | routes.rs:95                    | ✅          | 0003-api-reference.md:L864           |
| Endpoint | `POST /documents/scan`                          | routes.rs:100                   | ✅          | 0003-api-reference.md:L987           |
| Endpoint | `POST /documents/reprocess`                     | routes.rs:102                   | ✅          | 0003-api-reference.md:L1036          |
| Endpoint | `GET /documents/{document_id}`                  | routes.rs:104                   | ✅          | 0003-api-reference.md:L929           |
| Endpoint | `GET /documents/tasks/{track_id}`               | routes.rs:105                   | ✅          | 0003-api-reference.md:L959           |
| Endpoint | `POST /query`                                   | routes.rs:110                   | ✅          | 0003-api-reference.md:L1083          |
| Endpoint | `POST /query/stream`                            | routes.rs:111                   | ✅          | 0003-api-reference.md:L1160          |
| Endpoint | `GET /graph`                                    | routes.rs:113                   | ✅          | 0003-api-reference.md:L1202          |
| Endpoint | `GET /graph/nodes/{node_id}`                    | routes.rs:114                   | ✅          | 0003-api-reference.md:L1253          |
| Endpoint | `GET /graph/labels/search`                      | routes.rs:115                   | ✅          | 0003-api-reference.md:L1262          |
| Endpoint | `GET /graph/labels/popular`                     | routes.rs:116                   | ✅          | 0003-api-reference.md:L1290          |
| Endpoint | `POST /graph/entities`                          | routes.rs:118                   | ✅          | 0003-api-reference.md:L1328          |
| Endpoint | `GET /graph/entities/exists`                    | routes.rs:119                   | ✅          | 0003-api-reference.md:L1373          |
| Endpoint | `POST /graph/entities/merge`                    | routes.rs:120                   | ✅          | 0003-api-reference.md:L1431          |
| Endpoint | `GET /graph/entities/{entity_name}`             | routes.rs:121                   | ✅          | 0003-api-reference.md:L1393          |
| Endpoint | `PUT /graph/entities/{entity_name}`             | routes.rs:122                   | ✅          | 0003-api-reference.md:L1402          |
| Endpoint | `DELETE /graph/entities/{entity_name}`          | routes.rs:126                   | ✅          | 0003-api-reference.md:L1422          |
| Endpoint | `POST /graph/relationships`                     | routes.rs:131                   | ✅          | 0003-api-reference.md:L1471          |
| Endpoint | `GET /graph/relationships/{relationship_id}`    | routes.rs:132                   | ✅          | 0003-api-reference.md:L1494          |
| Endpoint | `PUT /graph/relationships/{relationship_id}`    | routes.rs:136                   | ✅          | 0003-api-reference.md:L1498          |
| Endpoint | `DELETE /graph/relationships/{relationship_id}` | routes.rs:140                   | ✅          | 0003-api-reference.md:L1502          |
| Endpoint | `GET /tasks/{track_id}`                         | routes.rs:145                   | ✅          | 0003-api-reference.md:L1551          |
| Endpoint | `GET /tasks`                                    | routes.rs:146                   | ✅          | 0003-api-reference.md:L1510          |
| Endpoint | `POST /tasks/{track_id}/cancel`                 | routes.rs:147                   | ✅          | 0003-api-reference.md:L1560          |
| Endpoint | `POST /tasks/{track_id}/retry`                  | routes.rs:148                   | ✅          | 0003-api-reference.md:L1569          |
| Endpoint | `GET /pipeline/status`                          | routes.rs:150                   | ✅          | 0003-api-reference.md:L1584          |
| Endpoint | `POST /pipeline/cancel`                         | routes.rs:151                   | ✅          | 0003-api-reference.md:L1625          |
| Config   | `storage.database_url`                          | config.rs:38                    | ✅          | 0007-configuration-reference.md:L101 |
| Config   | `storage.max_connections`                       | config.rs:40                    | ✅          | 0007-configuration-reference.md:L104 |
| Config   | `storage.min_connections`                       | config.rs:42                    | ✅          | 0007-configuration-reference.md:L107 |
| Config   | `storage.connect_timeout_secs`                  | config.rs:44                    | ✅          | 0007-configuration-reference.md:L110 |
| Config   | `storage.namespace`                             | config.rs:46                    | ✅          | 0007-configuration-reference.md:L113 |
| Config   | `llm.provider`                                  | config.rs:65                    | ✅          | 0007-configuration-reference.md:L154 |
| Config   | `llm.api_key`                                   | config.rs:67                    | ✅          | 0007-configuration-reference.md:L157 |
| Config   | `llm.base_url`                                  | config.rs:69                    | ✅          | 0007-configuration-reference.md:L160 |
| Config   | `llm.model`                                     | config.rs:71                    | ✅          | 0007-configuration-reference.md:L163 |
| Config   | `llm.embedding_model`                           | config.rs:73                    | ✅          | 0007-configuration-reference.md:L166 |
| Config   | `llm.embedding_dim`                             | config.rs:75                    | ✅          | 0007-configuration-reference.md:L169 |
| Config   | `llm.max_tokens`                                | config.rs:77                    | ✅          | 0007-configuration-reference.md:L175 |
| Config   | `llm.temperature`                               | config.rs:79                    | ✅          | 0007-configuration-reference.md:L172 |
| Config   | `llm.timeout_secs`                              | config.rs:81                    | ✅          | 0007-configuration-reference.md:L178 |
| Config   | `llm.max_retries`                               | config.rs:83                    | ✅          | 0007-configuration-reference.md:L181 |
| Config   | `pipeline.chunk_size`                           | config.rs:107                   | ✅          | 0007-configuration-reference.md:L234 |
| Config   | `pipeline.chunk_overlap`                        | config.rs:109                   | ✅          | 0007-configuration-reference.md:L237 |
| Config   | `pipeline.entity_types`                         | config.rs:111                   | ✅          | 0007-configuration-reference.md:L240 |
| Config   | `pipeline.max_entities_per_chunk`               | config.rs:113                   | ✅          | 0007-configuration-reference.md:L243 |
| Config   | `pipeline.max_relations_per_chunk`              | config.rs:115                   | ✅          | 0007-configuration-reference.md:L246 |
| Config   | `pipeline.summarize_descriptions`               | config.rs:117                   | ✅          | 0007-configuration-reference.md:L249 |
| Config   | `pipeline.max_description_tokens`               | config.rs:119                   | ✅          | 0007-configuration-reference.md:L252 |
| Config   | `pipeline.concurrency`                          | config.rs:121                   | ✅          | 0007-configuration-reference.md:L255 |
| Config   | `query.default_mode`                            | config.rs:151                   | ✅          | 0007-configuration-reference.md:L284 |
| Config   | `query.max_vector_results`                      | config.rs:153                   | ✅          | 0007-configuration-reference.md:L287 |
| Config   | `query.max_graph_depth`                         | config.rs:155                   | ✅          | 0007-configuration-reference.md:L290 |
| Config   | `query.max_context_entities`                    | config.rs:157                   | ✅          | 0007-configuration-reference.md:L293 |
| Config   | `query.max_context_relationships`               | config.rs:159                   | ✅          | 0007-configuration-reference.md:L296 |
| Config   | `query.max_context_chunks`                      | config.rs:161                   | ✅          | 0007-configuration-reference.md:L299 |
| Config   | `query.stream_responses`                        | config.rs:163                   | ✅          | 0007-configuration-reference.md:L302 |
| Config   | `api.host`                                      | config.rs:215                   | ✅          | 0007-configuration-reference.md:L54  |
| Config   | `api.port`                                      | config.rs:217                   | ✅          | 0007-configuration-reference.md:L57  |
| Config   | `api.cors_enabled`                              | config.rs:219                   | ✅          | 0007-configuration-reference.md:L60  |
| Config   | `api.cors_origins`                              | config.rs:221                   | ✅          | 0007-configuration-reference.md:L63  |
| Config   | `api.auth_enabled`                              | config.rs:223                   | ✅          | 0007-configuration-reference.md:L66  |
| Config   | `api.api_keys`                                  | config.rs:225                   | ✅          | 0007-configuration-reference.md:L69  |
| Config   | `api.body_limit`                                | config.rs:227                   | ✅          | 0007-configuration-reference.md:L72  |
| Config   | `api.timeout_secs`                              | config.rs:229                   | ✅          | 0007-configuration-reference.md:L75  |
| Type     | `QueryMode`                                     | query.rs:6                      | ✅          | 0007-configuration-reference.md:L305 |
| Type     | `DocumentStatus`                                | document.rs:14                  | ✅          | 0003-api-reference.md:L885           |
| Type     | `TaskStatus`                                    | tasks/types.rs:11               | ✅          | 0003-api-reference.md:L1515          |
| Type     | `TaskType`                                      | tasks/types.rs:39               | ✅          | 0003-api-reference.md:L1515          |
| Type     | `Role`                                          | auth/types.rs:14                | ✅          | 0003-api-reference.md:L425           |
| Type     | `Permission`                                    | auth/rbac.rs:8                  | ✅          | 0008-multi-tenancy.md:L340           |
| Storage  | `Memory`                                        | adapters/memory                 | ✅          | 0004-storage-backends.md:L100        |
| Storage  | `Postgres`                                      | adapters/postgres               | ✅          | 0004-storage-backends.md:L150        |
| Example  | `basic_rag.rs`                                  | examples/basic_rag.rs           | ✅          | 0001-quick-start.md:L100             |
| Example  | `graph_exploration.rs`                          | examples/graph_exploration.rs   | ✅          | 0001-quick-start.md:L405             |
| Example  | `multi_tenant.rs`                               | examples/multi_tenant.rs        | ✅          | 0008-multi-tenancy.md:L10            |
| Example  | `production_pipeline.rs`                        | examples/production_pipeline.rs | ✅          | 0001-quick-start.md:L360             |
| Example  | `streaming_query.rs`                            | examples/streaming_query.rs     | ✅          | 0001-quick-start.md:L406             |

## 1. File Inventory

| File                              | Lines | Read Status | Content Hash |
| :-------------------------------- | :---- | :---------- | :----------- |
| `0001-quick-start.md`             | 438   | ⏳          |              |
| `0002-architecture-overview.md`   | 579   | ⏳          |              |
| `0003-api-reference.md`           | 1754  | ⏳          |              |
| `0004-storage-backends.md`        | 778   | ⏳          |              |
| `0005-llm-integration.md`         | 544   | ⏳          |              |
| `0006-deployment-guide.md`        | 600   | ⏳          |              |
| `0007-configuration-reference.md` | 503   | ⏳          |              |
| `0008-multi-tenancy.md`           | 368   | ⏳          |              |
| `0009-algorithms-reference.md`    | 632   | ⏳          |              |
| `production-llm-integration.md`   | 515   | ⏳          |              |
| `README.md`                       | 198   | ⏳          |              |

## 2. Findings Log (Docs→Code Verification)

| Doc ID | Claim                                                                  | Source of Truth (File:Line)     | Status      | Action                         |
| :----- | :--------------------------------------------------------------------- | :------------------------------ | :---------- | :----------------------------- |
| F001   | `GET /health`                                                          | routes.rs:15                    | ✅ Verified | None                           |
| F002   | `GET /ready`                                                           | routes.rs:16                    | ✅ Verified | None                           |
| F003   | `GET /live`                                                            | routes.rs:17                    | ✅ Verified | None                           |
| F004   | `GET /metrics`                                                         | routes.rs:19                    | ✅ Verified | None                           |
| F005   | `GET /api/version`                                                     | routes.rs:33                    | ✅ Verified | None                           |
| F006   | `GET /api/tags`                                                        | routes.rs:34                    | ✅ Verified | None                           |
| F007   | `GET /api/ps`                                                          | routes.rs:35                    | ✅ Verified | None                           |
| F008   | `POST /api/generate`                                                   | routes.rs:36                    | ✅ Verified | None                           |
| F009   | `POST /api/chat`                                                       | routes.rs:37                    | ✅ Verified | None                           |
| F010   | `POST /api/v1/auth/login`                                              | routes.rs:44                    | ✅ Verified | None                           |
| F011   | `POST /api/v1/auth/refresh`                                            | routes.rs:45                    | ✅ Verified | None                           |
| F012   | `POST /api/v1/auth/logout`                                             | routes.rs:46                    | ✅ Verified | None                           |
| F013   | `GET /api/v1/auth/me`                                                  | routes.rs:47                    | ✅ Verified | None                           |
| F014   | `POST /api/v1/users`                                                   | routes.rs:49                    | ✅ Verified | None                           |
| F015   | `GET /api/v1/users`                                                    | routes.rs:50                    | ✅ Verified | None                           |
| F016   | `GET /api/v1/users/{user_id}`                                          | routes.rs:51                    | ✅ Verified | None                           |
| F017   | `DELETE /api/v1/users/{user_id}`                                       | routes.rs:52                    | ✅ Verified | None                           |
| F018   | `POST /api/v1/api-keys`                                                | routes.rs:54                    | ✅ Verified | None                           |
| F019   | `GET /api/v1/api-keys`                                                 | routes.rs:55                    | ✅ Verified | None                           |
| F020   | `DELETE /api/v1/api-keys/{key_id}`                                     | routes.rs:56                    | ✅ Verified | None                           |
| F021   | `POST /api/v1/tenants`                                                 | routes.rs:58                    | ✅ Verified | None                           |
| F022   | `GET /api/v1/tenants`                                                  | routes.rs:59                    | ✅ Verified | None                           |
| F023   | `GET /api/v1/tenants/{tenant_id}`                                      | routes.rs:60                    | ✅ Verified | None                           |
| F024   | `PUT /api/v1/tenants/{tenant_id}`                                      | routes.rs:61                    | ✅ Verified | None                           |
| F025   | `DELETE /api/v1/tenants/{tenant_id}`                                   | routes.rs:62                    | ✅ Verified | None                           |
| F026   | `POST /api/v1/tenants/{tenant_id}/workspaces`                          | routes.rs:64                    | ✅ Verified | None                           |
| F027   | `GET /api/v1/tenants/{tenant_id}/workspaces`                           | routes.rs:68                    | ✅ Verified | None                           |
| F028   | `GET /api/v1/workspaces/{workspace_id}`                                | routes.rs:72                    | ✅ Verified | None                           |
| F029   | `PUT /api/v1/workspaces/{workspace_id}`                                | routes.rs:73                    | ✅ Verified | None                           |
| F030   | `DELETE /api/v1/workspaces/{workspace_id}`                             | routes.rs:77                    | ✅ Verified | None                           |
| F031   | `GET /api/v1/workspaces/{workspace_id}/stats`                          | routes.rs:81                    | ✅ Verified | None                           |
| F032   | `POST /api/v1/documents`                                               | routes.rs:86                    | ✅ Verified | None                           |
| F033   | `POST /api/v1/documents/upload`                                        | routes.rs:94                    | ✅ Verified | None                           |
| F034   | `POST /api/v1/documents/upload/batch`                                  | routes.rs:95                    | ✅ Verified | None                           |
| F035   | `GET /api/v1/documents`                                                | routes.rs:87                    | ✅ Verified | None                           |
| F036   | `GET /api/v1/documents/{document_id}`                                  | routes.rs:104                   | ✅ Verified | None                           |
| F037   | `DELETE /api/v1/documents/{document_id}`                               | routes.rs:89                    | ✅ Verified | None                           |
| F038   | `GET /api/v1/documents/track/{track_id}`                               | routes.rs:105                   | ✅ Verified | None                           |
| F039   | `POST /api/v1/documents/scan`                                          | routes.rs:100                   | ✅ Verified | None                           |
| F040   | `POST /api/v1/documents/reprocess`                                     | routes.rs:102                   | ✅ Verified | None                           |
| F041   | `POST /api/v1/query`                                                   | routes.rs:110                   | ✅ Verified | None                           |
| F042   | `POST /api/v1/query/stream`                                            | routes.rs:111                   | ✅ Verified | None                           |
| F043   | `GET /api/v1/graph`                                                    | routes.rs:113                   | ✅ Verified | None                           |
| F044   | `GET /api/v1/graph/nodes/{node_id}`                                    | routes.rs:114                   | ✅ Verified | None                           |
| F045   | `GET /api/v1/graph/labels/search`                                      | routes.rs:115                   | ✅ Verified | None                           |
| F046   | `GET /graph/labels/popular`                                            | routes.rs:116                   | ❌ Mismatch | Doc missing `/api/v1` prefix   |
| F047   | `POST /api/v1/graph/entities`                                          | routes.rs:118                   | ✅ Verified | None                           |
| F048   | `GET /api/v1/graph/entities/exists`                                    | routes.rs:119                   | ✅ Verified | None                           |
| F049   | `GET /api/v1/graph/entities/{entity_name}`                             | routes.rs:121                   | ✅ Verified | None                           |
| F050   | `PUT /api/v1/graph/entities/{entity_name}`                             | routes.rs:122                   | ✅ Verified | None                           |
| F051   | `DELETE /api/v1/graph/entities/{entity_name}`                          | routes.rs:126                   | ✅ Verified | None                           |
| F052   | `POST /api/v1/graph/entities/merge`                                    | routes.rs:120                   | ✅ Verified | None                           |
| F053   | `POST /api/v1/graph/relationships`                                     | routes.rs:131                   | ✅ Verified | None                           |
| F054   | `GET /api/v1/graph/relationships/{relationship_id}`                    | routes.rs:132                   | ✅ Verified | None                           |
| F055   | `PUT /api/v1/graph/relationships/{relationship_id}`                    | routes.rs:136                   | ✅ Verified | None                           |
| F056   | `DELETE /api/v1/graph/relationships/{relationship_id}`                 | routes.rs:140                   | ✅ Verified | None                           |
| F057   | `GET /api/v1/tasks`                                                    | routes.rs:146                   | ✅ Verified | None                           |
| F058   | `GET /api/v1/tasks/{track_id}`                                         | routes.rs:145                   | ✅ Verified | None                           |
| F059   | `POST /api/v1/tasks/{track_id}/cancel`                                 | routes.rs:147                   | ✅ Verified | None                           |
| F060   | `POST /api/v1/tasks/{track_id}/retry`                                  | routes.rs:148                   | ✅ Verified | None                           |
| F061   | `GET /api/v1/pipeline/status`                                          | routes.rs:150                   | ✅ Verified | None                           |
| F062   | `POST /api/v1/pipeline/cancel`                                         | routes.rs:151                   | ✅ Verified | None                           |
| F063   | `api.host` default "0.0.0.0"                                           | config.rs:215                   | ✅ Verified | None                           |
| F064   | `api.port` default 8080                                                | config.rs:217                   | ✅ Verified | None                           |
| F065   | `api.cors_enabled` default true                                        | config.rs:219                   | ✅ Verified | None                           |
| F066   | `api.cors_origins` default ["*"]                                       | config.rs:221                   | ✅ Verified | None                           |
| F067   | `api.auth_enabled` default false                                       | config.rs:223                   | ✅ Verified | None                           |
| F068   | `api.api_keys` default []                                              | config.rs:225                   | ✅ Verified | None                           |
| F069   | `api.body_limit` default 10MB                                          | config.rs:227                   | ✅ Verified | None                           |
| F070   | `api.timeout_secs` default 300                                         | config.rs:229                   | ✅ Verified | None                           |
| F071   | `storage.database_url` default "postgres://localhost:5432/edgequake"   | config.rs:38                    | ✅ Verified | None                           |
| F072   | `storage.max_connections` default 10                                   | config.rs:40                    | ✅ Verified | None                           |
| F073   | `storage.min_connections` default 1                                    | config.rs:42                    | ✅ Verified | None                           |
| F074   | `storage.connect_timeout_secs` default 30                              | config.rs:44                    | ✅ Verified | None                           |
| F075   | `storage.namespace` default None                                       | config.rs:46                    | ✅ Verified | None                           |
| F076   | `llm.provider` default "openai"                                        | config.rs:65                    | ✅ Verified | None                           |
| F077   | `llm.model` default "gpt-4o-mini"                                      | config.rs:71                    | ✅ Verified | None                           |
| F078   | `llm.embedding_model` default "text-embedding-3-small"                 | config.rs:73                    | ✅ Verified | None                           |
| F079   | `llm.embedding_dim` default 1536                                       | config.rs:75                    | ✅ Verified | None                           |
| F080   | `llm.temperature` default 0.0                                          | config.rs:79                    | ✅ Verified | None                           |
| F081   | `llm.max_tokens` default 4096                                          | config.rs:77                    | ✅ Verified | None                           |
| F082   | `llm.timeout_secs` default 60                                          | config.rs:81                    | ✅ Verified | None                           |
| F083   | `llm.max_retries` default 3                                            | config.rs:83                    | ✅ Verified | None                           |
| F084   | `llm.api_key` default None                                             | config.rs:67                    | ✅ Verified | None                           |
| F085   | `llm.base_url` default None                                            | config.rs:69                    | ✅ Verified | None                           |
| F086   | `pipeline.chunk_size` default 1200                                     | config.rs:107                   | ✅ Verified | None                           |
| F087   | `pipeline.chunk_overlap` default 100                                   | config.rs:109                   | ✅ Verified | None                           |
| F088   | `pipeline.entity_types` default [...]                                  | config.rs:111                   | ✅ Verified | None                           |
| F089   | `pipeline.max_entities_per_chunk` default 20                           | config.rs:113                   | ✅ Verified | None                           |
| F090   | `pipeline.max_relations_per_chunk` default 20                          | config.rs:115                   | ✅ Verified | None                           |
| F091   | `pipeline.summarize_descriptions` default true                         | config.rs:117                   | ✅ Verified | None                           |
| F092   | `pipeline.max_description_tokens` default 1200                         | config.rs:119                   | ✅ Verified | None                           |
| F093   | `pipeline.concurrency` default 4                                       | config.rs:121                   | ✅ Verified | None                           |
| F094   | `query.default_mode` default Hybrid                                    | config.rs:151                   | ✅ Verified | None                           |
| F095   | `query.max_vector_results` default 20                                  | config.rs:153                   | ✅ Verified | None                           |
| F096   | `query.max_graph_depth` default 3                                      | config.rs:155                   | ✅ Verified | None                           |
| F097   | `query.max_context_entities` default 30                                | config.rs:157                   | ✅ Verified | None                           |
| F098   | `query.max_context_relationships` default 30                           | config.rs:159                   | ✅ Verified | None                           |
| F099   | `query.max_context_chunks` default 20                                  | config.rs:161                   | ✅ Verified | None                           |
| F100   | `query.stream_responses` default true                                  | config.rs:163                   | ✅ Verified | None                           |
| F101   | `QueryMode` variants: Naive, Local, Global, Hybrid, Bypass             | config.rs:183                   | ✅ Verified | None                           |
| F102   | `DocumentStatus` variants: Pending, Processing, Processed, Failed      | document.rs:14                  | ✅ Fixed    | Updated 0003-api-reference.md  |
| F103   | `TaskStatus` variants: Pending, Processing, Indexed, Failed, Cancelled | tasks/types.rs:11               | ✅ Fixed    | Updated 0003-api-reference.md  |
| F104   | `TaskType` variants: Upload, Insert, Scan, Reindex                     | tasks/types.rs:39               | ✅ Verified | None                           |
| F105   | `Role` variants: Admin, User, Readonly                                 | auth/types.rs:14                | ✅ Verified | None                           |
| F106   | `Permission` variants                                                  | auth/rbac.rs:8                  | ✅ Fixed    | Added to 0008-multi-tenancy.md |
| F107   | `Memory` storage adapter                                               | adapters/memory                 | ✅ Verified | None                           |
| F108   | `Postgres` storage adapter                                             | adapters/postgres               | ✅ Verified | None                           |
| F109   | `basic_rag.rs` example                                                 | examples/basic_rag.rs           | ✅ Verified | None                           |
| F110   | `graph_exploration.rs` example                                         | examples/graph_exploration.rs   | ✅ Fixed    | Added to 0001-quick-start.md   |
| F111   | `multi_tenant.rs` example                                              | examples/multi_tenant.rs        | ✅ Verified | None                           |
| F112   | `production_pipeline.rs` example                                       | examples/production_pipeline.rs | ✅ Verified | None                           |
| F113   | `streaming_query.rs` example                                           | examples/streaming_query.rs     | ✅ Fixed    | Added to 0001-quick-start.md   |

## 3. Coverage Gaps (Code→Docs Missing)

| Feature                | Type    | Source                        | Severity | Action                         |
| :--------------------- | :------ | :---------------------------- | :------- | :----------------------------- |
| `Permission`           | Type    | auth/rbac.rs:8                | ✅ Fixed | Added to 0008-multi-tenancy.md |
| `graph_exploration.rs` | Example | examples/graph_exploration.rs | ✅ Fixed | Added to 0001-quick-start.md   |
| `streaming_query.rs`   | Example | examples/streaming_query.rs   | ✅ Fixed | Added to 0001-quick-start.md   |

## 4. Ambiguities & Blockers

- [ ] _None_

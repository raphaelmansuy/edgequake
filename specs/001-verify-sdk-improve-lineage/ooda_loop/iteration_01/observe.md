# OODA Iteration 01 — Observe: Full SDK Baseline Assessment

**Date**: 2026-02-13  
**Mission**: SDK Quality Assurance & Lineage Enhancement  
**Focus**: Map territory — all 10 SDKs, 133 backend routes

## Backend API Surface

Total routes in `edgequake/crates/edgequake-api/src/routes.rs`: **133 `.route()` calls**

### Route Categories (27 groups)

| Category           | Routes | Key Endpoints                                              |
|--------------------|--------|------------------------------------------------------------|
| Health             | 4      | /health, /ready, /live, /metrics                           |
| WebSocket          | 2      | /ws/pipeline/progress, /ws/progress/{track_id}             |
| Ollama Emulation   | 5      | /api/version, /api/tags, /api/ps, /api/generate, /api/chat |
| Auth               | 4      | login, refresh, logout, me                                 |
| Users              | 4      | create, list, get, delete                                  |
| API Keys           | 3      | create, list, revoke                                       |
| Tenants            | 5      | create, list, get, update, delete                          |
| Workspaces         | 10     | CRUD, stats, metrics-history, snapshot, rebuilds            |
| Documents          | 15     | CRUD, upload, batch, scan, reprocess, lineage, metadata    |
| PDF                | 8      | upload, list, get, delete, progress, retry, cancel, content|
| Query              | 2      | query, stream                                              |
| Chat               | 2      | completions, stream                                        |
| Conversations      | 10     | CRUD, import, bulk ops, messages, share                    |
| Messages           | 2      | update, delete                                             |
| Folders            | 4      | list, create, update, delete                               |
| Shared             | 1      | get shared conversation                                    |
| Graph              | 6      | get, stream, nodes, labels                                 |
| Entities           | 7      | CRUD, exists, merge, neighborhood                          |
| Relationships      | 4      | CRUD                                                       |
| Tasks              | 4      | get, list, cancel, retry                                   |
| Pipeline           | 3      | status, cancel, queue-metrics                              |
| Costs              | 6      | pricing, estimate, summary, history, budget get/patch      |
| Lineage            | 2      | entity lineage, document lineage                           |
| Chunks             | 2      | detail, lineage                                            |
| Provenance         | 1      | entity provenance                                          |
| Settings           | 2      | provider status, list providers                            |
| Models             | 5      | list all, llm, embedding, health, per-provider             |

## SDK Inventory

### Python SDK (`sdks/python/`)
- **Structure**: `edgequake/resources/` with 7 resource files + `_base.py`
- **Resources**: auth, chat, conversations, documents, graph, operations, query
- **Types**: 10 type modules (auth, chat, conversations, documents, graph, operations, query, shared, workspaces)
- **Tests**: 15 test files, **6,229 lines total**
- **Coverage**: sync + async variants for all resources
- **Metadata/Lineage**: `get_lineage()`, `get_metadata()`, `export_lineage()` methods present

### TypeScript SDK (`sdks/typescript/`)
- **Structure**: `src/resources/` with 22 resource files
- **Resources**: api-keys, auth, chat, chunks, conversations, costs, documents, folders, graph, lineage, models, ollama, pipeline, provenance, query, settings, shared, tasks, tenants, users, workspaces
- **Tests**: 3 dirs (e2e/, helpers/, unit/), **4,753 lines total**
- **Coverage**: Most comprehensive resource set (22 files matching ~22 categories)

### Rust SDK (`sdks/rust/`)
- **Structure**: `src/resources/` with 22 resource files
- **Resources**: api_keys, auth, chat, chunks, conversations, costs, documents, entities, folders, graph, health, models, pdf, pipeline, provenance, query, relationships, tasks, tenants, users, workspaces
- **Types**: 10 type modules
- **Tests**: 3 test files, **2,251 lines total**

### C# SDK (`sdks/csharp/`)
- **Structure**: `src/EdgeQuakeSDK/` — monolithic `Services.cs` + `Models.cs`
- **Tests**: `E2ETest.cs`, `UnitTest.cs`, `MockHttpMessageHandler.cs` — **857 lines**
- **Pattern**: Single `EdgeQuakeClient` with service methods

### Go SDK (`sdks/go/`)
- **Structure**: Flat package — `client.go`, `services.go`, `types.go`
- **Tests**: `edgequake_test.go`, `e2e_test.go`, `edgequake_coverage_test.go` — **3,294 lines**
- **Pattern**: Single `Client` struct with methods

### Java SDK (`sdks/java/`)
- **Structure**: `src/main/java/` package hierarchy
- **Tests**: **1,413 lines** across test files
- **Pattern**: `EdgeQuakeClient` with service classes

### Kotlin SDK (`sdks/kotlin/`)
- **Structure**: `src/main/kotlin/` package hierarchy
- **Tests**: **1,249 lines** across test files
- **Pattern**: Coroutine-based `EdgeQuakeClient`

### PHP SDK (`sdks/php/`)
- **Structure**: `src/` with Client, Services, Config, HttpHelper, ApiError
- **Tests**: `E2ETest.php`, `UnitTest.php`, `MockHttpHelper.php` — **1,063 lines**

### Ruby SDK (`sdks/ruby/`)
- **Structure**: `lib/edgequake/` with client, services, config, http_helper, error
- **Tests**: `e2e_test.rb`, `unit_test.rb`, `mock_http_helper.rb` — **725 lines**

### Swift SDK (`sdks/swift/`)
- **Structure**: `Sources/EdgeQuakeSDK/` with Client, Services, Models, Config, HttpHelper, Error
- **Tests**: `Tests/EdgeQuakeSDKTests/` — **843 lines**
- **Pattern**: Swift Package Manager project

## Initial Coverage Estimates

| SDK        | Test LOC | Resource Files | API Coverage Est. | Metadata/Lineage |
|------------|----------|----------------|-------------------|------------------|
| Python     | 6,229    | 7 + types      | ~85%              | ✅ Full          |
| TypeScript | 4,753    | 22             | ~90%              | ✅ Full          |
| Rust       | 2,251    | 22             | ~85%              | ✅ Full          |
| C#         | 857      | 2 (monolithic) | ~60%              | ⚠️ Partial       |
| Go         | 3,294    | 2 (flat)       | ~65%              | ⚠️ Partial       |
| Java       | 1,413    | varies         | ~50%              | ❌ Missing       |
| Kotlin     | 1,249    | varies         | ~50%              | ❌ Missing       |
| PHP        | 1,063    | 2 (monolithic) | ~55%              | ⚠️ Partial       |
| Ruby       | 725      | 2 (monolithic) | ~60%              | ⚠️ Partial       |
| Swift      | 843      | 2 (monolithic) | ~50%              | ❌ Missing       |

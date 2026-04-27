# SPEC-009 — 100 OODA cycles (registry)

**Format:** Each ID is one **Observe → Orient → Decide → Act** iteration. **Act** is the engineering delta (code, test, or doc). Status is implied **open** until merged; update in git history, not by editing this table ad nauseam.

**Precondition:** Read [SDK-PRE-ASSESSMENT.md](./SDK-PRE-ASSESSMENT.md) before starting any ID.

| ID | SDK | Observe | Act |
|----|-----|---------|-----|
| 001 | meta | Single canon doc drifts from `routes.rs` | Regenerate [API-CANON.md](./API-CANON.md) when routes change; CI grep for phantom list |
| 002 | meta | SDKs invent `page_size` vs `per_page` | Standardize on **`page_size`** for list endpoints that use it in Rust handlers |
| 003 | meta | Merge entity uses mixed keys | **`source_name`/`target_name`** only for `/graph/entities/merge` |
| 004 | meta | Relationship create uses mixed keys | **`src_id`/`tgt_id`/`keywords` (string)** + `description` + `source_id` + `weight` |
| 005 | meta | Revoke API key | **`DELETE`** `/api-keys/{id}` only |
| 006 | meta | User update | **No** `PUT /users/{id}` — remove client methods |
| 007 | meta | Message edit | **`PATCH /messages/{message_id}`** not under conversations nest |
| 008 | meta | Readiness/liveness | **`/ready` `/live`** not `/health/ready` |
| 009 | meta | Workspace list | **Tenant-scoped** `GET /tenants/{tid}/workspaces` |
| 010 | meta | Shared read | **`GET /shared/{share_id}`** only |
| 011 | Swift | `/health/ready` | Use `/ready` |
| 012 | Swift | `/health/live` | Use `/live` |
| 013 | Swift | `/health/detailed` phantom | Delete `detailed()` |
| 014 | Swift | `PUT /documents/{id}` | Delete `update()` |
| 015 | Swift | `GET /documents/search` | Delete `search()` |
| 016 | Swift | `GET /documents/{id}/chunks` | Delete `chunks()` |
| 017 | Swift | `GET /documents/{id}/status` | Delete `status()` |
| 018 | Swift | `POST /documents/{id}/reprocess` | Replace with **`POST /documents/reprocess`** |
| 019 | Swift | Missing track | Add **`GET /documents/track/{id}`** |
| 020 | Swift | Entity `types()` | Delete |
| 021 | Swift | Relationship wrong POST body | Fix **`src_id`/`tgt_id`/keywords** |
| 022 | Swift | Relationship missing GET | Add **`get(id:)`** |
| 023 | Swift | Relationship `types()` | Delete |
| 024 | Swift | Graph `stats()` | Delete |
| 025 | Swift | Graph `clear()` | Delete |
| 026 | Swift | Graph `neighbors` wrong path | Delete; use entity neighborhood server-side |
| 027 | Swift | Graph `subgraph` | Delete |
| 028 | Swift | Graph missing `GET /graph/nodes/{id}` | Add `getNode` |
| 029 | Swift | Graph missing label list shape | Add **`GET /labels/search?q=`** decode `[String]` |
| 030 | Swift | Graph missing `degrees/batch` | Add `degreesBatch` |
| 031 | Swift | Auth login email | Use **`username`+`password`** JSON |
| 032 | Swift | Auth `change-password` | Delete |
| 033 | Swift | User `update` | Delete |
| 034 | Swift | User `create` shape | **username, email, password** |
| 035 | Swift | ApiKey `get` | Delete |
| 036 | Swift | ApiKey `rotate` | Delete |
| 037 | Swift | Task `create` | Delete |
| 038 | Swift | Task `status` subpath | Delete; use **`get(id:)`** |
| 039 | Swift | Pipeline pause/resume/processing/config | Delete phantoms |
| 040 | Swift | Pipeline cancel | Keep only if matches `POST /pipeline/cancel` |
| 041 | Swift | Model `list` `/models/list` | Delete |
| 042 | Swift | Model `providers` wrong path | Map to **`/settings/providers`** |
| 043 | Swift | Model `providerHealth` | Delete |
| 044 | Swift | Model `setDefault` / `test` | Delete |
| 045 | Swift | Model provider GET | Align **`/models/{provider}`** |
| 046 | Swift | Cost daily/by/export | Delete phantoms; keep summary/history if routed |
| 047 | Swift | Conversation `PUT` update | Use **`PATCH`** |
| 048 | Swift | Message delete path | **`DELETE /messages/{id}`** |
| 049 | Swift | Conversation search/export | Delete phantoms |
| 050 | Swift | Folder `GET/{id}` | Delete if not routed |
| 051 | Swift | Folder `PUT` update | Use **`PATCH`** |
| 052 | Swift | Folder move/conversations | Delete phantoms |
| 053 | Swift | Workspace `GET /workspaces` list | Replace with **`list(tenantId:)`** |
| 054 | Swift | Workspace `switch` | Delete |
| 055 | Swift | Shared links API | Replace **`SharedService`** with **`get(shareId:)`** |
| 056 | Kotlin | Entity `types()` | Delete |
| 057 | Kotlin | Relationship `types()` | Delete |
| 058 | Kotlin | Merge JSON keys | **`source_name`/`target_name`** |
| 059 | Kotlin | `CreateRelationshipRequest` | Align **`src_id`/`tgt_id`/keywords** |
| 060 | Kotlin | Graph label JSON | **`labels: [string]`** |
| 061 | Kotlin | Graph batch degrees | **`degrees: [{node_id, degree}]`** |
| 062 | Kotlin | User `update` | Delete |
| 063 | Kotlin | ApiKey `get` | Delete |
| 064 | Kotlin | ApiKey `rotate` | Delete |
| 065 | Kotlin | ApiKey `create` extras | **`name` only** per handler |
| 066 | Kotlin | Auth `changePassword` | Delete |
| 067 | Kotlin | Auth `refresh` body | **`refresh_token`** required |
| 068 | Kotlin | Conversation `PUT` | **`PATCH`** for conversation |
| 069 | Kotlin | Message delete path | **`/messages/{id}`** |
| 070 | Kotlin | Conversation search | Delete if not routed |
| 071 | Kotlin | Folder extras | Trim to list/create/**PATCH**/delete |
| 072 | Kotlin | Task create/status | Delete phantoms |
| 073 | Kotlin | Pipeline pause/resume/processing | Delete phantoms |
| 074 | Kotlin | Model fantasy methods | Trim to catalog/health/providerStatus + real model paths |
| 075 | Kotlin | Cost budget verb | **`PATCH`** not POST |
| 076 | C# | User update | Remove if still present |
| 077 | C# | Conversation PATCH | Audit `ConversationService` |
| 078 | C# | Model service paths | Line-by-line vs canon |
| 079 | Java | Model/Task/Pipeline | Same audit |
| 080 | Java | Admin/effective client | Add or document absent |
| 081 | Go | Multipart injection | Add `PutInjectionFile` helper or doc gap |
| 082 | Go | List query param audit | Grep `per_page` |
| 083 | PHP | Admin service | Add PATCH quota + defaults |
| 084 | PHP | Effective config | Add `GET /config/effective` |
| 085 | PHP | Tenant workspace list | Add helper |
| 086 | Rust | WS client | Optional `ws/progress/{track}` client |
| 087 | Python | WS client | Same |
| 088 | TS | WS client | Same |
| 089 | Ruby | Missing `lib/` | **Done:** `sdks/ruby/lib/edgequake` + services aligned to canon |
| 090 | Ruby | Tests vs canon | **Done:** unit tests use `/ready`, `/live`, `/metrics`, canonical document/graph/relationship paths |
| 091 | Ruby | Document phantoms | **Done:** removed non-canonical document client methods + tests |
| 092 | Ruby | Graph phantoms | **Done:** removed types/stats/neighbors-style tests; canonical graph helpers |
| 093 | meta | E2E matrix | [IMPLEMENTATION-PROOF.md](./IMPLEMENTATION-PROOF.md) owns commands |
| 094 | meta | Ollama nest | SDKs optional; paths under `/api/*` |
| 095 | meta | Error shape | Map Axum `ApiError` JSON consistently |
| 096 | meta | Idempotency | DELETE/PUT semantics documented per handler |
| 097 | meta | Security | Never log API keys; redact in debug |
| 098 | meta | Versioning | All business under `/api/v1` |
| 099 | meta | DRY imports | Shared path constants file per SDK (optional) |
| 100 | meta | Closure | Re-run pre-assessment; Tier A must stay green on CI |

**Note:** IDs 011–055 and 056–075 are **Swift** and **Kotlin** batching; executing all **Act** items may require multiple PRs. The registry is the backlog; git commits are the source of truth for completion.

# SPEC-009 — OODA improvement log (SDK quality)

Ten **Observe → Orient → Decide → Act** cycles applied against `routes.rs` / [API-CANON.md](./API-CANON.md). Each cycle removes phantom surfaces or aligns verbs, paths, and DTOs with the server.

| # | Observe | Orient | Decide | Act |
|---|---------|--------|--------|-----|
| **1** | PHP exposed `GET /documents/{id}/chunks`, `…/status`, `PATCH …/metadata` | Listed in API-CANON “not routed” | Drop phantoms; use `GET /documents/{id}` + track endpoints | `DocumentService`: removed chunks/status/setMetadata; added `track()`, fixed PDF paths |
| **2** | PHP `GET /graph/stats`, `POST /graph/clear` absent from router | First principle: no client methods without handlers | Delete | `GraphService`: removed `stats()`, `clear()` |
| **3** | PHP entity/relationship “types” URLs not in `routes.rs` | Same as invented taxonomy endpoints | Remove | Dropped `EntityService::types()`, `RelationshipService::types()`; added `exists()` |
| **4** | PHP relationship `POST` body used wrong shape | Handler expects `CreateRelationshipRequest` (`src_id`, `tgt_id`, `keywords` string, …) | Align body | `RelationshipService::create()` with canonical fields |
| **5** | PHP `POST …/api-keys/{id}/revoke` not routed | Revocation is `DELETE /api-keys/{key_id}` | Single HTTP story | `revoke()` → `DELETE`; kept alias semantics |
| **6** | PHP message PATCH/DELETE under `conversations/.../messages` | Router uses `/messages/{message_id}` | Fix paths | `ConversationService` message methods |
| **7** | PHP `PUT /users/{id}` not in API | User update not exposed | Remove | Dropped `UserService::update()` + tests |
| **8** | C# `GraphService.StatsAsync`, `label=` query, relationship path `/source/target`, merge `primary_id` | All diverge from `routes.rs` | Align graph + entities + relationships | Removed stats; `q=` for label search; relationship by id; merge `source_name`/`target_name`; `ExistsAsync` |
| **9** | Java `stats`, `clear`, `types`, label/degree JSON shapes vs API; list queries used `per_page` | Phantom endpoints + wrong DTOs + query param drift | Remove phantoms; fix models; align pagination | `GraphService` trimmed; label/degrees/popular models; entity/relationship types removed; `page_size` on documents / entities / relationships / tasks lists |
| **10** | Docs claimed PHP “FAIL” for chunk surface; Tier B C# “relationship path shapes” | Assessment stale after Act | Update tier copy | [SDK-QUALITY-ASSESSMENT.md](./SDK-QUALITY-ASSESSMENT.md) + this log |

**SOLID / DRY:** Services stay one-per-resource; shared truth remains `API-CANON.md` + `routes.rs` — no duplicate “convenience” routes that the server does not implement.

**Follow-up (not closed here):** Ruby `lib/` tree missing from repo; Swift still carries many non-canonical paths (`SharedService`, `TaskService`, `ModelService` subsets); Kotlin cost verb / shared paths; full Tier B model audits.

# SPEC-009 — Brutal honest SDK quality & completeness

**Single source of truth:** `edgequake/crates/edgequake-api/src/routes.rs` (+ handler DTOs).  
**Scale:** **PASS** = matches most `/api/v1/*` surfaces used in production; typed models; tests prove paths. **FAIL** = phantom routes, wrong HTTP verbs, or systematic drift.

---

## Tier A — **PASS** (aligned + tested)

| SDK | Verdict | Notes |
|-----|---------|--------|
| **Rust** (`sdks/rust`) | **PASS** | Routes, admin, effective config, workspace injections, chunk/provenance shapes updated; `cargo test -p edgequake-sdk` is the contract. |
| **Python** (`sdks/python`) | **PASS** | Parity work on provenance, admin, multipart PUT, async/sync; large `pytest` suite. |
| **TypeScript / Node** (`sdks/typescript`) | **PASS** | `@edgequake/sdk`; admin, effectiveConfig, injection paths; `bun test` green. |

---

## Tier B — **PARTIAL** (usable but incomplete or historically drifted)

| SDK | Grade | Brutally honest summary |
|-----|-------|-------------------------|
| **Go** (`sdks/go`) | **B+** | Paths largely verified against `routes.rs`; **fixed in SPEC-009:** tenant workspace list now uses `{ items }` wrapper; `put`/`patch` on client; `Admin`, `EffectiveCfg`, `PutInjection`. Still no multipart file injection helper; some responses stay loosely typed (`map[string]interface{}` for admin). |
| **C#** (`sdks/csharp`) | **B−** | **Fixed in OODA batch:** phantom `GraphService.StatsAsync` removed; label search uses `q=`; relationships addressed by `{relationship_id}`; `CreateAsync` sends `src_id`/`tgt_id`/string `keywords`; entity merge uses `source_name`/`target_name`; `EntityService.ExistsAsync`. **Still audit:** `ModelService`, `TaskService`, `PipelineService`, `UserService.Update`, `ConversationService` extras vs `routes.rs`. |
| **Java** (`sdks/java`) | **B−** | **Fixed:** removed graph `stats`/`clear`, entity/relationship `types`; label search + batch degrees + popular labels models closer to API JSON. **Remaining:** `EntityService.list` query params (`per_page` vs `page_size`), admin/effective, model/task/pipeline fantasy methods; local Maven may need corporate repo bypass to run tests. |
| **Kotlin** (`sdks/kotlin`) | **C** | **Fixed:** documents (track + reprocessFailed), graph (removed phantom stats/clear), workspaces (tenant-scoped list/create, rebuildEmbeddings, injection list/get shape loose), admin + effectiveConfig. **Remaining:** relationship paths, cost budget verb (POST vs PATCH), `SharedService` paths, graph `batchDegrees` response shape vs API (`degrees: []`), many DTOs still fantasy. |

---

## Tier C — **FAIL** (not yet brought to `routes.rs`)

| SDK | Grade | Summary |
|-----|-------|---------|
| **PHP** (`sdks/php`) | **C+** | **Fixed:** removed document chunks/status/metadata PATCH, graph stats/clear, entity/relationship types, user `PUT`; API-key revoke via `DELETE`; messages via `/messages/{id}`; PDF + `track()` canonical; `RelationshipService::create()` body aligned. **Remaining:** no `Admin` / effective config; workspace tenant-scoped listing; PHPUnit is the contract (`./vendor/bin/phpunit`). |
| **Ruby** (`sdks/ruby`) | **FAIL** | `upload_text` / chunks / status assumptions; README examples call non-canonical paths. |
| **Swift** (`sdks/swift`) | **FAIL** | Same class of phantom document chunk/status helpers; no admin surface. |

---

## OODA loop (how we drive this to green)

1. **Observe:** diff SDK path strings vs `API-CANON.md`.  
2. **Orient:** classify phantom vs missing vs wrong verb/body.  
3. **Decide:** one PR per SDK family; shared checklist (documents, workspaces, admin, graph degrees batch).  
4. **Act:** delete phantoms first (safe), then add missing admin/config/injection, then tighten types.

**Quality gate definition:** Tier A = PASS. Tier B = mergeable with documented gaps. Tier C = not advertised as “complete” until chunk/admin audits land.

---

## Cross-references

- [API-CANON.md](./API-CANON.md)  
- [RUST-SDK-GAP.md](./RUST-SDK-GAP.md), [python-skd/GAP.md](./python-skd/GAP.md), [node-js/GAP.md](./node-js/GAP.md)  
- [IMPLEMENTATION-PROOF.md](./IMPLEMENTATION-PROOF.md)  

# SPEC-009 — Pre-OODA brutal SDK assessment

**Authority:** `edgequake/crates/edgequake-api/src/routes.rs` + handler DTOs.  
**Date:** 2026-04-24.  
**Rule:** No marketing language. If a method calls a path that returns **404 on a stock server**, it is a **defect**, not a “future feature”.

## Methodology (repeatable)

1. **Route groups** (12): health, auth, users/keys, tenants/workspaces/admin, documents, query/chat, conversations/messages/folders/shared, graph, tasks/pipeline/costs, lineage/chunks/provenance, models/settings/config, ollama emulation, websockets.
2. **Score per group:** `0` = mostly phantom or wrong verb; `1` = partial; `2` = usable against real API; `3` = typed + tests prove paths.
3. **Overall** = sum / 36 (max 1.0). Rounded **tier**: A ≥ 0.78, B ≥ 0.50, else C.
4. **Phantom index (PI):** count of public SDK methods whose first HTTP call does not exist in `routes.rs` (estimated; spot-checked with ripgrep).

## Scorecard (before next OODA wave)

| SDK | PI (est.) | Sum/36 (est.) | Tier | Brutal summary |
|-----|-----------|----------------|------|----------------|
| **Rust** | low | ~0.92 | **A** | Closest to law-of-the-server; integration tests are the contract. Residual risk: optional routes added in API before SDK PR. |
| **Python** | low | ~0.90 | **A** | Broad coverage; some loosely typed admin/config corners. |
| **TypeScript** | low | ~0.88 | **A** | Barrel + e2e; drift possible on niche document/pdf subpaths. |
| **Go** | low–med | ~0.72 | **B** | Solid core; multipart injection and some admin shapes still `interface{}`. |
| **C#** | med | ~0.62 | **B** | Graph/relationship/entity batch fixed; **Model/Task/Pipeline/User/Conversation** still need line-by-line `routes.rs` diff. |
| **Java** | med | ~0.58 | **B** | Pagination + graph models improved; **phantom services** (many Kotlin-parity copies) likely remain on Model/Task/Pipeline; Maven may be blocked on corporate mirrors. |
| **Kotlin** | **high** | ~0.48 | **C** | Merge DTO used wrong JSON keys; relationship **create** DTO wrong; **entity/relationship types** were phantom; **ApiKey get/rotate**, **User PUT**, **auth change-password**, **conversation/folder/task/pipeline/model** extras are mostly fantasy until deleted or rewritten. |
| **PHP** | med | ~0.55 | **C+** | Core documents/graph/auth cleaned; **no admin / effective config / tenant workspace list**. |
| **Swift** | **high** | ~0.35 | **C** | Widespread phantom paths (`/health/ready`, document chunk/status/search/update, graph stats/neighbors/subgraph, workspace list without tenant, shared “links”, model/task/pipeline noise). **Must be treated as unsafe for production** until stripped to canon. |
| **Ruby** | **critical** | ~0.05 | **F** | **`lib/` not shipped in tree** — tests are orphaned. Even if restored, unit tests assert non-canonical URLs. Not a product. |

## Coverage dimensions (binary)

| Dimension | Rust | Py | TS | Go | C# | Java | Kotlin | PHP | Swift | Ruby |
|---------|------|----|----|----|----|------|--------|-----|-------|------|
| Health `/ready` `/live` not under `/health/*` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | **✗** | **✗** |
| Documents: no `…/chunks` `…/status` | ✓ | ✓ | ✓ | ? | ✓ | ✓ | ✓ | ✓ | **✗** | **✗** |
| Graph: no `stats` / `types` | ✓ | ✓ | ✓ | ? | ✓ | ✓ | **fixing** | **fixing** | **✗** | **✗** |
| Relationships: `{id}` not `src/tgt` path | ✓ | ✓ | ✓ | ? | ✓ | ✓ | ✓ | ✓ | **✗** | ? |
| Messages: `PATCH/DELETE /messages/{id}` | ✓ | ✓ | ✓ | ? | ? | ✓ | **✗** | ✓ | **✗** | ? |
| Workspaces: tenant-scoped list | ✓ | ✓ | ✓ | ✓ | ? | ? | ✓ | **✗** | **✗** | ? |
| Admin + `/config/effective` | ✓ | ✓ | ✓ | ? | partial | **✗** | partial | **✗** | **✗** | ? |

## Engineering principles (non-negotiable)

- **First principles:** If it is not in the router table, it does not exist. Clients must not encode product fiction.
- **SOLID:** One service class per **routed** resource; delete “god” helpers that mix 3 APIs.
- **DRY:** Pagination query name is **`page_size`** unless a specific handler documents otherwise; merge bodies use **`source_name`/`target_name`** for entities; relationship create uses **`src_id`/`tgt_id`/`keywords` (string)**.

## Where the bodies are buried

- **Kotlin / Swift:** copy-paste “enterprise” method expansion without OpenAPI lockstep.
- **Ruby:** missing `lib/` — tests are **lies without implementation**.
- **Tier A:** regressions only when `routes.rs` changes without SDK CI.

## Next actions

Execute [OODA-100-REGISTRY.md](./OODA-100-REGISTRY.md) in order; after each **Act**, re-run this assessment’s ripgrep probes and update PI.

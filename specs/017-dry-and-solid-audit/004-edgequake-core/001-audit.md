# edgequake-core — DRY & SOLID Audit

**Crate path:** `edgequake/crates/edgequake-core`  
**LOC:** ~14,576 (src)  
**Role:** Orchestration facade, domain types, services, **legacy in-crate query engine**

---

## Executive Summary

**Split-brain query execution** is the defining architectural debt. Production API routes through `edgequake-query::SOTAQueryEngine`, but `EdgeQuake::initialize()` constructs `crate::query::QueryEngine` (~1,145 LOC of parallel implementation). Comments claim SOTA delegation; code does not. Four separate `QueryMode` enums and two `StorageConfig` structs create latent type bugs.

---

## DRY Violations

| ID | P | Violation | Evidence | Remediation |
|----|---|-----------|----------|-------------|
| CORE-DRY-001 | **P0** | Dual query engines (~1,145 LOC) | `src/query/{naive,local,global,hybrid,mix,bypass}.rs`; orchestrator `mod.rs:488-496` uses core engine; API uses SOTA | Deprecate `core/query/`; wire orchestrator to `edgequake-query` |
| CORE-DRY-002 | **P0** | Hybrid semantics differ | Core: round-robin (`query/hybrid.rs:29-40`); query crate: dedupe concat (`strategies/hybrid.rs:61-92`) | Single engine = single behavior |
| CORE-DRY-003 | **P0** | Four `QueryMode` enums | `config.rs:175`, `types/query.rs:6`, `edgequake-query/modes.rs:55`, `types/conversation/enums.rs:10` | One canonical enum in core; aliases elsewhere |
| CORE-DRY-004 | **P0** | `StorageConfig` name collision | `config.rs:24-35` (DB pool) vs `orchestrator/mod.rs:194-203` (backend enum); `lib.rs:97` re-exports orchestrator version | Rename: `DatabasePoolConfig` vs `OrchestratorStorageConfig` |
| CORE-DRY-005 | **P1** | Triple config model | `config.rs`, `EdgeQuakeConfig`, `QueryEngineConfig` — overlapping fields, no mapper | `EdgeQuakeConfig::from(Config)` facade |
| CORE-DRY-006 | **P1** | Entity type defaults 3× | Pipeline: 9 types; `config.rs:118-129` inline; `EdgeQuakeConfig` default: 5 types | `edgequake_pipeline::default_entity_types()` everywhere |
| CORE-DRY-007 | **P1** | LLM model defaults scattered | `gpt-4.1-nano` vs `gpt-4.1-mini` across config/types/orchestrator | Single constants module |
| CORE-DRY-008 | **P1** | Keyword extraction duplicated | `keyword_extractor.rs` vs `edgequake-query/keywords/` | Delete core copy; depend on query crate |
| CORE-DRY-009 | **P1** | Token budget unused in query modes | `token_budget.rs` exists; core `query/` modes don't use it | Wire or delete |
| CORE-DRY-010 | **P2** | `matches_tenant` filter 4× | `query/mod.rs:64-101`, `edgequake-query/sota_engine/prompt.rs:17-44` | Shared filter helper in core types |
| CORE-DRY-011 | **P2** | Graph node→`ContextEntity` mapping copy-pasted | `orchestrator/query_ops.rs:219-239`, `292-311`, `331-351` | `fn node_to_context_entity(...)` |
| CORE-DRY-012 | **P2** | Entity normalization (3 algorithms) | Pipeline title-case; query `UPPERCASE`; core query: none shared | Use pipeline normalizer |
| CORE-DRY-013 | **P2** | Dead `utils/text.rs` | Zero production imports; only self-tests | Delete or move to shared crate |
| CORE-DRY-014 | **P2** | Parallel context types | `ContextEntity` vs `RetrievedEntity` vs `MessageContextEntity` | Conversion helpers |

---

## SOLID Violations

| ID | P | Principle | Violation | Evidence |
|----|---|-----------|-----------|----------|
| CORE-SOLID-S-001 | **P0** | SRP | `EdgeQuake` god orchestrator | Owns config, 3 storages, providers, pipeline, query engine |
| CORE-SOLID-S-002 | **P2** | SRP | `workspace_service.rs` ~1,010 LOC monolith | Split trait / in_memory / factory |
| CORE-SOLID-S-003 | **P1** | SRP | Crate scope creep | Types + orchestrator + query + services + token budget |
| CORE-SOLID-O-001 | **P0** | OCP | New query mode → N-way edits | Match in `query/mod.rs:108-115` + 6 impl files + query crate |
| CORE-SOLID-L-001 | **P0** | LSP | Core vs SOTA engines not substitutable | Hybrid behavior differs |
| CORE-SOLID-I-001 | **P2** | ISP | Fat `ConversationService` (15+ methods) | Split repositories |
| CORE-SOLID-I-002 | **P2** | ISP | Fat `WorkspaceService` | Split tenant vs workspace vs metrics |
| CORE-SOLID-D-001 | **P1** | DIP | Orchestrator constructs concrete pipeline types | No extractor trait boundary |
| CORE-SOLID-D-002 | **P0** | DIP | Cycle avoidance → wrong abstraction | Comment `orchestrator/mod.rs:103`; dev-dep only on query |

---

## Critical Evidence

Orchestrator comment vs reality:

```488:496:edgequake/crates/edgequake-core/src/orchestrator/mod.rs
        // Initialize SOTA query engine from edgequake-query
        let query_engine = crate::query::QueryEngine::new(
            llm.clone(),
            embedding.clone(),
            graph_storage.clone(),
            vector_storage.clone(),
        );
```

`edgequake-query` is **dev-dependency only** in `Cargo.toml` — not linked in production orchestrator path.

---

## Remediation Plan

### P0 — Architectural integrity

1. Add `edgequake-query` as production dependency OR extract `edgequake-query-types` to break cycle
2. Wire `EdgeQuake::initialize()` → `SOTAQueryEngine`
3. Delete or thin-wrap `src/query/` (mark `#[deprecated]`)
4. Unify `QueryMode`; rename colliding `StorageConfig`
5. Fix misleading comments in `query_ops.rs`, `orchestrator/mod.rs`

### P1 — Config consolidation

6. Single config facade with env loading for pipeline/query sections
7. Centralize defaults (entity types, LLM models)
8. Consolidate keyword extraction + token budget

### P2 — Module hygiene

9. Split `workspace_service.rs`; graph mapping helper
10. Delete dead `utils/text.rs`
11. Complete or remove orchestrator stubs (`get_document`, `list_documents`)

---

## Verification

```bash
cargo test -p edgequake-core --lib
# After migration: grep must show zero production use of crate::query::QueryEngine
rg 'crate::query::QueryEngine' edgequake/crates/edgequake-core/src --glob '!**/query/**'
```

**Acceptance:** Orchestrator and API share one query engine; hybrid mode tests identical across paths.

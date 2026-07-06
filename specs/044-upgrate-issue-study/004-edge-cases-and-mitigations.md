# SPEC-044 — Edge Cases & Mitigations (Battle-Tested Register)

Exhaustive failure modes for the v0.14.x upgrade ingest incident, the Cypher bind regression, and the fix plan. Each row maps to a verification gate in [008-implementation-plan.md](./008-implementation-plan.md).

---

## <a id="p0"></a>P0 — Fix `cypher_*_bound` (bare `$1` + text bind)

| # | Edge case | Risk | Mitigation | Battle test |
| - | --------- | ---- | ---------- | ------------- |
| 1 | `$1::agtype` cast as third arg | AGE rejects cast expression | Use **bare `$1`** only | BT-044-01: negative SQL probe |
| 2 | `sqlx::query().bind(serde_json::Value)` sends jsonb | No jsonb→agtype cast in AGE | Bind `String` (JSON text); let `agtype_in` parse | BT-044-02: `spec022` injection test |
| 3 | `sqlx::raw_sql` skips prepared statement protocol | `$1` not recognized | Use `sqlx::query(&sql).bind(...)` not `raw_sql` for bound paths | BT-044-03: execute + query bound |
| 4 | Inline `'…'::agtype` (current v0.14.0) | `must be a parameter` | Remove Mode C entirely | BT-044-04: grep gate — no `params_lit` in bound SQL |
| 5 | Empty params map `{}` | No-op MATCH miss | Valid; delete idempotent on missing node | BT-044-05: delete absent node |
| 6 | `node_id` contains `'` or Cypher special chars | Injection if escaped wrong | Cypher `$node_id` param in map; no string interpolation in query body | BT-044-06: `INJECT' OR 1=1 --` in spec022 |
| 7 | `node_id` contains Unicode / newlines | Parse errors in agtype map | `serde_json` serialization; integration test | BT-044-07: unicode node id |
| 8 | Very long `node_id` / description in param map | Oversized bind | Within PG max param size; chunk not applicable for single-key delete | BT-044-08: 4KB node_id |
| 9 | Dollar-quote collision in cypher body | SQL parse error | Existing `dollar_quote_tag()` — reuse | BT-044-09: cypher containing `$$` |
| 10 | PG16 AGE 1.6.0 vs PG17/18 AGE 1.7.0 | Divergent behaviour | Same Mode B contract on both; CI matrix PG16+PG18 | BT-044-10: `make spec042-battle-test-all` subset |
| 11 | Connection pool: `SET LOCAL` leaked | Wrong search_path | `setup_age_session_scoped` per acquire; existing pattern | BT-044-11: pool stress 50 delete cycles |
| 12 | Concurrent delete same node | Harmless race | DETACH DELETE idempotent | BT-044-12: parallel delete_node |

---

## <a id="p1"></a>P1 — Fallback: inline `cypher_execute` for delete paths

| # | Edge case | Risk | Mitigation | Battle test |
| - | --------- | ---- | ---------- | ------------- |
| 1 | `escape_cypher_string` bypass | SQL injection | Route all ids through `escape_cypher_string`; fuzz quotes/backslash | BT-044-13: injection fuzz |
| 2 | Divergence bound vs fallback semantics | Test drift | Prefer P0; P1 only as emergency backport | BT-044-14: both paths delete same node |
| 3 | Scoped delete uses inline; unscoped uses bound | Inconsistent | After P0, unify; until then align unscoped to scoped pattern | BT-044-15: compare pg_delete_node vs scoped |

---

## <a id="comp"></a>Compensation saga (`compensate_merge_failure`)

| # | Edge case | Risk | Mitigation | Battle test |
| - | --------- | ---- | ---------- | ------------- |
| 1 | Merge `errors=1` from unrelated entity; C1236 persisted | **Deletes good nodes** | P2: phase-scoped artifacts | BT-044-16: induced single-entity build fail |
| 2 | Relationship batch fails after entity success | Rolls back valid entities | P2: only rollback relationship artifacts + placeholders | BT-044-17: mock edge upsert fail |
| 3 | Compensation delete fails (pre-fix) | Orphan nodes + vectors | P0 fix; quarantine log for manual cleanup | BT-044-18: compensation integration test |
| 4 | Compensation vector delete fails | Orphan embeddings | Existing quarantine log; reconciliation job | BT-044-19: `compensation::tests` |
| 5 | `graph_nodes_created` includes placeholder UNKNOWN nodes | Rollback removes placeholders | Expected; verify rel batch failure path | BT-044-20: relationship merge test |
| 6 | Re-ingest same document after failed compensation | Duplicate merge attempt | Idempotent MERGE on node_id | BT-044-21: double ingest E2E |
| 7 | Original error masked by compensation panic | Lost operability | Compensation never panics; never returns Err to caller | BT-044-22: code audit compensation.rs |
| 8 | Empty artifacts | No-op compensation | Early return paths | BT-044-23: `compensate_noop_on_empty` |

---

## <a id="merge"></a>Primary merge failure modes (upgrade context)

| # | Edge case | Risk | Mitigation | Battle test |
| - | --------- | ---- | ---------- | ------------- |
| 1 | M043 AGE upgrade lock on first boot | Transient batch failure | Retry ingest; `/health` shows extversion; maintenance window | BT-044-24: bootstrap logs |
| 2 | Concurrent index build (large graph) | Slow writes, timeout | Non-fatal bootstrap; retry | BT-044-25: `bootstrap_concurrent_indexes` |
| 3 | LLM summarizer timeout on one entity | `stats.errors += 1` | Fallback to simple merge (already); consider not counting as hard error | BT-044-26: mock LLM fail |
| 4 | `get_nodes_batch` on missing label (SPEC-039) | `relation does not exist` | `ensure_graph_labels` — should not recur on upgrade | BT-044-27: `post_upgrade_health.sql` |
| 5 | Native writes flag `EDGEQUAKE_NATIVE_GRAPH_WRITES=1` | Different upsert path | Test both native and Cypher batch paths | BT-044-28: env matrix in E2E script |
| 6 | Halfvec / HNSW dim guard (M071) | Vector upsert fail before merge | Separate from this incident; `/ready` gates | BT-044-29: health embedding_dim |
| 7 | Tenant-scoped graph with missing `tenant_id` on legacy nodes | Merge/update skew | Backfill migration; scoped ops | BT-044-30: multi-tenant ingest |
| 8 | Document with 0 entities, N relationships | Placeholder-only nodes | relationship.rs placeholder path | BT-044-31: rel-only fixture |
| 9 | Document with entities, 0 relationships | Entity-only path | entity.rs batch path | BT-044-32: entity-only fixture |
| 10 | Worker retry same PDF (`existing_document_id`) | Double compensation | Single-flight admission (SPEC-021 RC-19) | BT-044-33: worker retry test |

---

## <a id="upgrade"></a>Upgrade / migration edge cases

| # | Edge case | Risk | Mitigation | Battle test |
| - | --------- | ---- | ---------- | ------------- |
| 1 | App v0.14.x on PG16 volume (AGE 1.6.0) | Cypher bind bug | P0 fix | BT-044-10 PG16 profile |
| 2 | App v0.14.x on PG18 volume (AGE 1.7.0) | Same bind bug | P0 fix | BT-044-10 PG18 profile |
| 3 | Skip image rebuild; app-only bump | Old extension catalog | M042/M043 on boot | BT-044-34: extversion check SQL |
| 4 | `edgequakeSchema.sql`-only validation | False confidence on AGE | Run `post_upgrade_health.sql` | BT-044-35: operator doc |
| 5 | Rollback app to v0.13.x on v0.14 schema | Forward-only migrations | Document: app downgrade OK, schema not | BT-044-36: CHANGELOG rollback note |
| 6 | Orphan nodes after failed compensation (pre-fix) | Graph pollution | Reconciliation: `entity_reconcile` / manual delete | BT-044-37: orphan detector query |
| 7 | First ingest during extension UPDATE | Intermittent failure | Retry; monitor `migration_043` logs | BT-044-38: deploy runbook |

---

## Cross-cutting verification gates (release blocker)

Before closing SPEC-044:

1. `cargo test -p edgequake-storage --features postgres --test spec022_cypher_prepared_postgres` — **green** (not skip).
2. `./specs/044-upgrate-issue-study/e2e/run_upgrade_ingest_cypher_proof.sh` — exit 0.
3. `psql -f specs/044-upgrate-issue-study/e2e/sql/post_upgrade_health.sql` — all checks PASS.
4. `cargo test -p edgequake-storage --lib compensation` — green.
5. `cargo test -p edgequake-pipeline --lib merger` — green.
6. Grep gate: `cypher_execute_bound` must not contain `'::agtype)` third-arg literal pattern.
7. Induced merge-failure test: compensation deletes node without quarantine log.
8. PG16 **and** PG18 CI matrix (SPEC-042 REQ-042C-06).

---

## <a id="downstream"></a>Downstream callers (C-6 — same bug, different symptoms)

| # | Call site | Broken op | Symptom if unfixed | Battle test |
| - | --------- | --------- | ------------------ | ----------- |
| 1 | `compensation.rs` | `delete_node`, `delete_edge` | Quarantine log; orphan graph | BT-044-18, D-1 |
| 2 | `orchestrator/deletion.rs` | `delete_node`, `delete_edge` | Document delete incomplete | BT-044-44, D-3 |
| 3 | `entity_reconcile.rs` | `get_node`, `get_edge`, `delete_*` | Reconcile partial failure | BT-044-46, D-4 |
| 4 | `entity_graph_lookup.rs` | `get_node` | Entity API 404 | BT-044-45, D-5 |
| 5 | `entity_ops.rs` resolve | `get_node` | GET/merge/neighborhood 404 | D-5 |
| 6 | `orchestrator/query_ops.rs` | `get_node` | Query missing entity context | BT-044-47, D-7 |
| 7 | `handlers/isolation.rs` | `get_node` | Tenant isolation false negative | D-5 |
| 8 | `graph_batch_contract` | `has_node` | CI contract fail (masked) | D-8 |
| 9 | `postgres_integration` CRUD | all six bound ops | Integration test fail (masked) | D-9 |

**Note:** `delete_node_scoped` / `delete_edge_scoped` use inline Cypher (Mode A) — **not affected**. Entity merge API works; compensation does not.

---

## <a id="ci"></a>CI & test false confidence (C-7, C-8)

| # | Edge case | Risk | Mitigation | Battle test |
| - | --------- | ---- | ---------- | ----------- |
| 1 | `postgres-integration.yml` `continue-on-error: true` on graph contracts | Known C-1 ships | Remove mask after P0a | BT-044-42 |
| 2 | `spec022_cypher_exec` asserts `contains("::agtype")` | Locks in broken pattern | Invert to `!contains("params_lit}'::agtype")` | BT-044-43 |
| 3 | spec022 skip says `DATABASE_URL` but checks `POSTGRES_PASSWORD` | Operators think test ran | Fix message; derive creds from URL | BT-044-48 |
| 4 | Static spec022 tests pass without Postgres | False green on every PR | Integration test required in CI | BT-044-02 |
| 5 | `postgres_integration` age CRUD `continue-on-error` | delete_node failure invisible | Required gate post-fix | D-9 |
| 6 | v0.14.0 commit "allow AGE batch contract failure" | Process acceptance of bug | Revert policy in SPEC-044 | BT-044-42 |

---

## <a id="session"></a>Session / connection edge cases (adjacent)

| # | Edge case | Risk | Mitigation | Battle test |
| - | --------- | ---- | ---------- | ----------- |
| 1 | `cypher_execute_bound` uses `raw_sql` not `query` | `$1` never bound (C-3) | `sqlx::query().bind()` on acquired conn | BT-044-03 |
| 2 | `LOAD; SET; SELECT` in one prepare string | Prepare may fail | Session setup via typed queries first | BT-044-39 |
| 3 | Bound reads skip `apply_age_tenant_rls_context` | Cross-tenant read when RLS on | Pass tenant into bound helpers | BT-044-40 |
| 4 | `cypher_query_bound` uses conn; execute used pool+raw | Inconsistent behaviour | Unify acquire+setup pattern | BT-044-11 |
| 5 | PgBouncer transaction pooling | Prepared stmt broken | Document; test with pooler optional | BT-044-41 |

---

## <a id="scoped"></a>Scoped vs unscoped asymmetry (C-5)

| # | Edge case | Risk | Mitigation | Battle test |
| - | --------- | ---- | ---------- | ----------- |
| 1 | API merge uses scoped delete; coordinator uses unscoped | Merge OK, doc delete fails | P0a fixes unscoped | BT-044-44 |
| 2 | Developer copies scoped pattern for reads | Reads still need bind fix | P0a or native SQL get | P3-1 |
| 3 | Tenant delete API vs system compensation | Different code paths diverge | Converge on one delete impl after P0a | BT-044-15 |

---

## <a id="triple-track"></a>Triple-track PG16 / PG17 / PG18 (BT-044-TT)

| # | Edge case | Risk | Mitigation | Battle test |
| - | --------- | ---- | ---------- | ----------- |
| 1 | Fix works on PG18 only (dev default) | pg16 prod still broken | `run_triple_track_cypher_proof.sh all` | BT-044-TT-08–10 ×3 |
| 2 | AGE 1.6.0 vs 1.7.0 prepare semantics differ | pg16-only failure | SQL PREPARE probes on all tiers | BT-044-TT-05–07 |
| 3 | Download page says PG17=1.6.0, ship 1.7.0 | operator confusion | Document in 007; gate `extversion` | BT-044-TT-02 |
| 4 | PG18 uuidv7 changes client assumptions | false tier conflation | Separate BT-044-TT-12 gate | BT-044-TT-12 |
| 5 | Port collision running 3 containers | flaky local CI | `host_port=55440+major` | script |
| 6 | Rust tests hit wrong container | false pass/fail | `POSTGRES_PORT` per profile | script |
| 7 | Image not built for pg17 | skip profile | `make postgres-image-build-pg17` | P0c-1 |
| 8 | SPEC-042 passes but Cypher bind fails | false confidence | Run SPEC-044 after SPEC-042 | release order |

Official: [AGE download](https://age.apache.org/download/), [Prepared Statements](https://age.apache.org/age-manual/master/advanced/prepared_statements.html), [extension-pins.sh](../../../edgequake/docker/extension-pins.sh).

---

## Battle test ID quick reference

| ID | Proves |
| -- | ------ |
| BT-044-01–12 | P0 Cypher bind correctness |
| BT-044-13–15 | P1 fallback safety |
| BT-044-16–23 | Compensation saga |
| BT-044-24–33 | Primary merge paths |
| BT-044-34–38 | Upgrade operator gates |
| BT-044-39–48 | Session, CI, downstream (audit §11) |
| BT-044-TT-01–14 | Triple-track per [007-triple-track-battle-test.md](./007-triple-track-battle-test.md) |
| D-1–D-9 | Downstream path E2E ([008 plan](./008-implementation-plan.md#phase-p0d--downstream-path-verification-)) |

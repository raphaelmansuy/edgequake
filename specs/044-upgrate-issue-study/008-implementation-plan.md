# SPEC-044 — Implementation Plan (Exhaustive)

**Target release:** v0.14.2 (patch)  
**Scope:** `edgequake-storage` Cypher layer, test/CI gates, downstream verification, doc drift  
**Audit:** [006-similar-issues-audit.md](./006-similar-issues-audit.md)

---

## Summary

| Phase | Focus | Items | Release blocker |
| ----- | ----- | ----- | --------------- |
| **P0a** | Fix `cypher_exec.rs` | 8 tasks | ✅ Yes |
| **P0b** | Fix test falsehoods | 6 tasks | ✅ Yes |
| **P0c** | CI hardening | 5 tasks | ✅ Yes |
| **P0d** | Downstream E2E proof | 9 tasks | ✅ Yes |
| **P1** | Fallback + ops | 4 tasks | Recommended |
| **P2** | Compensation scope + docs | 6 tasks | Follow-up |
| **P3** | Structural hardening | 4 tasks | Backlog |

---

## Phase P0a — Fix `cypher_exec.rs` (C-1, C-3) ⬜

**Single SSOT:** `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers/cypher_exec.rs`

### P0a-1: Rewrite `cypher_query_bound`

- [ ] Remove `escape_agtype_literal` / `params_lit` from bound paths (may keep for tests only if needed elsewhere — grep first)
- [ ] SQL shape:
  ```sql
  SELECT … FROM cypher('graph', $tag $cypher $tag, $1) AS (cols…)
  ```
- [ ] `let params_json = serde_json::to_string(params)?;`
- [ ] `sqlx::query(&sql).bind(params_json).fetch_all(&mut *conn).await?`
- [ ] Keep: `setup_age_session_scoped(&mut conn, None)` on acquired connection
- [ ] Keep: `dollar_quote_tag(cypher)` for body safety
- [ ] **Do not** use `$1::agtype` (C-2)
- [ ] **Do not** inline `'…'::agtype` (C-1)

### P0a-2: Rewrite `cypher_execute_bound`

- [ ] Same `$1` + text bind pattern
- [ ] **Replace `sqlx::raw_sql` with `sqlx::query`** (C-3)
- [ ] Connection pattern:
  ```rust
  let pool = self.pool.get().await?;
  let mut conn = pool.acquire().await?;
  Self::setup_age_session_scoped(&mut conn, None).await?;
  sqlx::query(&sql).bind(params_json).execute(&mut *conn).await?;
  ```
- [ ] **Remove** `age_session_setup_sql()` prefix from bound execute (setup via typed queries instead — avoids multi-statement + prepare interaction; see edge case BT-044-39)

### P0a-3: Optional shared helper (DRY)

- [ ] Extract `fn cypher_bound_sql(graph, cypher, columns, execute) -> String` returning SQL with bare `$1` (restore pre-v0.14.0 structure, fixed bind type)
- [ ] Single place for `$1` third-arg contract

### P0a-4: Module documentation

- [ ] Replace incorrect "inline agtype literal" justification with AGE prepared-statement contract + link to SPEC-044
- [ ] Document: bind `String` JSON, not `serde_json::Value`

### P0a-5: Tenant RLS hook (when E-02 enabled)

- [ ] Pass `tenant_id: Option<&str>` into bound helpers OR call `apply_age_tenant_rls_context` after session setup
- [ ] Battle test BT-044-40 when `EDGEQUAKE_AGE_RLS=1`

### P0a acceptance

```bash
cd edgequake
POSTGRES_PASSWORD=test_password_123 POSTGRES_DB=edgequake_test POSTGRES_USER=edgequake_test \
  cargo test -p edgequake-storage --features postgres \
  --test spec022_cypher_prepared_postgres -- --nocapture

POSTGRES_PASSWORD=... cargo test -p edgequake-storage --features postgres \
  --test postgres_integration test_postgres_age_graph_crud -- --nocapture
```

---

## Phase P0b — Fix test falsehoods (C-8) ⬜

### P0b-1: `spec022_cypher_prepared_postgres.rs`

- [ ] Fix skip message: `POSTGRES_PASSWORD not set` (not `DATABASE_URL`)
- [ ] Use `contract_postgres_config` consistently; optionally accept `DATABASE_URL` → derive password in test helper
- [ ] **Remove** `assert!(src.contains("::agtype"))` from `spec022_cypher_exec_exposes_bound_helpers`
- [ ] **Add** `assert!(!src.contains("params_lit}'::agtype"))` — forbids C-1
- [ ] **Add** `assert!(src.contains(", $1)"))` or bare `$1` in bound SQL builder
- [ ] **Add** `assert!(!src.contains("raw_sql"))` inside `cypher_execute_bound` block (or fn-level grep)

### P0b-2: `graph_batch_contract.rs`

- [ ] Already calls `has_node` — becomes real integration gate after P0a

### P0b-3: Add `spec044_compensation_postgres.rs` (new)

- [ ] Seed node via `upsert_node` (native/cypher inline)
- [ ] Call `compensation::compensate_orphan_graph_writes` with that node id
- [ ] Assert node deleted; **no** quarantine log (use tracing test subscriber or assert `delete_node` Ok)

### P0b-4: Add `spec044_deletion_coordinator_postgres.rs` (new, in edgequake-core or api)

- [ ] Insert doc + entity linked to doc chunks
- [ ] Run deletion coordinator
- [ ] Assert `get_node` None (uses fixed `get_node`)

### P0b-5: Negative SQL test in `post_upgrade_health.sql`

- [ ] Already has inline literal rejection probe — keep

### P0b-6: Update `006-similar-issues-audit.md` checklist when done

---

## Phase P0c — CI hardening + triple-track battle test (PG16/PG17/PG18) ⬜

**SSOT:** [007-triple-track-battle-test.md](./007-triple-track-battle-test.md)  
**Runner:** [`e2e/run_triple_track_cypher_proof.sh`](./e2e/run_triple_track_cypher_proof.sh)  
**SQL probes:** [`e2e/sql/cypher_param_contract.sql`](./e2e/sql/cypher_param_contract.sql)

### P0c-0: Triple-track matrix (official docs grounded)

| Profile | PG | AGE min | Official release | EdgeQuake pin |
| ------- | -- | ------- | ---------------- | ------------- |
| pg16 | 16 | 1.6.0 | [PG16/v1.6.0-rc0](https://github.com/apache/age/releases/tag/PG16%2Fv1.6.0-rc0) | `extension-pins.sh` |
| pg17 | 17 | 1.7.0 | [PG17/v1.7.0-rc0](https://github.com/apache/age/releases/tag/PG17%2Fv1.7.0-rc0) | `extension-pins.sh` |
| pg18 | 18 | 1.7.0 | [PG18/v1.7.0-rc0](https://github.com/apache/age/releases/tag/PG18%2Fv1.7.0-rc0) | `extension-pins.sh` |

Cypher contract: [AGE Prepared Statements](https://age.apache.org/age-manual/master/advanced/prepared_statements.html) — identical on 1.6.0 and 1.7.0.

### P0c-1: Makefile + local gate

- [ ] Add `make spec044-battle-test-all` → `run_triple_track_cypher_proof.sh all`
- [ ] Document in [000-index.md](./000-index.md)

```bash
make postgres-image-build postgres-image-build-pg17 postgres-image-build-pg18
make spec044-battle-test-all
```

### P0c-2: Per-profile probes (BT-044-TT-01 – TT-14)

| Probe | pg16 | pg17 | pg18 |
| ----- | ---- | ---- | ---- |
| SQL version + extversion gates | ✅ | ✅ | ✅ |
| Negative inline `::agtype` | ✅ | ✅ | ✅ |
| PREPARE/EXECUTE `$1` read/delete | ✅ | ✅ | ✅ |
| Rust spec022 injection CRUD | ✅ | ✅ | ✅ |
| Rust postgres_integration CRUD | ✅ | ✅ | ✅ |
| Rust storage_backend_contract | ✅ | ✅ | ✅ |
| uuidv7 absent / present | absent | absent | **present** |

Reports: `e2e/reports/{pg16,pg17,pg18}-cypher-report.txt`

### P0c-3: `.github/workflows/postgres-integration.yml`

- [ ] **Remove** `continue-on-error: true` on graph contract steps (~L268, ~L286)
- [ ] Add job `spec044-triple-track-cypher` matrix: `[pg16, pg17, pg18]`
- [ ] Or extend `postgres-age-tests` to run `run_triple_track_cypher_proof.sh all`

### P0c-4: `release-gates.yml` / `release-docker.yml`

- [ ] Require `make spec044-battle-test-all` before image publish
- [ ] Compose after SPEC-042: `spec042-battle-test-all` → `spec044-battle-test-all`

### P0c-5: Grep gate

```bash
! rg "params_lit\}'::agtype" edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers/cypher_exec.rs
```

### P0c-6: Integrate with SPEC-042 suite

- [ ] Add step to `specs/042-update-age-pgvector/e2e/run_all_battle_tests.sh` (optional step 6)

### P0c acceptance (all three profiles)

```bash
./specs/044-upgrate-issue-study/e2e/run_triple_track_cypher_proof.sh all
# Must pass on pg16 (AGE 1.6.0), pg17 (AGE 1.7.0), pg18 (AGE 1.7.0)
```

---

## Phase P0d — Downstream path verification ⬜

Each row maps to [006-similar-issues-audit.md §4](./006-similar-issues-audit.md#4-downstream-production-paths-c-6).

| ID | Path | Test | Priority |
| -- | ---- | ---- | -------- |
| D-1 | `compensation::compensate_orphan_graph_writes` | `spec044_compensation_postgres.rs` | P0 |
| D-2 | `ingestion_persister` + induced merge error | Extend `e2e_spec021_ingestion_persister` or new | P0 |
| D-3 | `orchestrator/deletion.rs` | `spec044_document_delete_graph_postgres.rs` | P0 |
| D-4 | `entity_reconcile::execute` | Unit with memory ✅; add postgres | P1 |
| D-5 | `entity_graph_lookup::lookup_entity_node_for_context` | API test `e2e_entity_lookup_postgres` | P0 |
| D-6 | `entity_ops` merge (scoped delete) | Existing — regression only | P1 |
| D-7 | `query_ops` get_node enrichment | `e2e_advanced_retrieval` postgres | P1 |
| D-8 | `graph_isp_contract` postgres | Un-skip / fix | P0 |
| D-9 | `backend_e2e_contract` / `storage_backend_contract` | Remove continue-on-error | P0 |

### Battle test script update

- [ ] Extend `e2e/run_upgrade_ingest_cypher_proof.sh` to run all P0d tests
- [ ] Fail if any `SKIP` when Postgres credentials present

---

## Phase P1 — Emergency fallback + ops ⬜

*Only if P0a blocked; otherwise defer.*

### P1-1: Align unscoped deletes to scoped pattern

- [ ] `pg_delete_node`: inline escape + `cypher_execute` (mirror scoped)
- [ ] `pg_delete_edge`: same

### P1-2: Reads remain broken without P0a

- [ ] `has_node` / `get_node` cannot use inline safely for **parameterized** Cypher `$node_id` without bind — **P0a required** for reads

### P1-3: Production reconciliation

- [ ] Run orphan detector SQL from [005-risk-analysis.md](./005-risk-analysis.md) post-deploy
- [ ] Re-ingest failed `document_id`s from Graylog

### P1-4: Loki alert

- [ ] Alert on `quarantine: failed to roll back orphan` > 0 / 15m

---

## Phase P2 — Compensation scope + documentation ⬜

### P2-1: Phase-scoped `MergeArtifacts`

- [ ] `entity_artifacts` vs `relationship_artifacts`
- [ ] On entity partial error: don't rollback unrelated new nodes (BT-044-16)
- [ ] On relationship error: rollback placeholders only (BT-044-17)

### P2-2: Doc drift cleanup

- [ ] `specs/022-edgequake-study/06-improvement-plan.md` — bare `$1`
- [ ] `specs/016-datalayer-audit/007-improvements/004-edge-cases-and-mitigations.md` SC1
- [ ] `specs/016-datalayer-audit/007-improvements/005-security-hardening.md`
- [ ] `specs/042-update-age-pgvector/013-version-feature-matrix-official-docs.md` — note fix version
- [ ] Cross-link SPEC-044 in SPEC-042 [009-cross-reference-matrix.md](../042-update-age-pgvector/009-cross-reference-matrix.md)

### P2-3: CHANGELOG v0.14.2

### P2-4: `docs/operations/configuration.md` — post-upgrade SQL pointer

---

## Phase P3 — Structural hardening (backlog) ⬜

### P3-1: Native SQL `get_node` by `node_id` (DRY with `pg_get_nodes_batch`)

- [ ] Eliminate bound Cypher for single-node read hot path
- [ ] Keep bound Cypher for edge pattern match OR native SQL on EDGE table

### P3-2: Unified graph op trait internal routing

- [ ] `GraphCypherStrategy` enum: Native | Inline | Bound — explicit per op

### P3-3: `storage_inspector` INV-C — parameterize doc_id

- [ ] Separate from C-1; reduce SQL injection surface

### P3-4: Deprecate `escape_agtype_literal` if unused after P0a

---

## Edge cases mitigated (full register)

See [004-edge-cases-and-mitigations.md](./004-edge-cases-and-mitigations.md) for BT-044-01–40.

### New edge cases from similar-issues audit

| ID | Edge case | Phase | Mitigation |
| -- | --------- | ----- | ---------- |
| BT-044-39 | Multi-statement `LOAD; SET; SELECT cypher…$1` fails prepare | P0a | Typed session setup on conn, then single-statement cypher query |
| BT-044-40 | AGE RLS tenant context missing on bound reads | P0a-5 | `apply_age_tenant_rls_context` |
| BT-044-41 | PgBouncer statement mode breaks prepare | P0c | CI with pooler optional job |
| BT-044-42 | CI continue-on-error hides regression | P0c | Remove mask |
| BT-044-43 | Static test requires `::agtype` | P0b | Invert assertion |
| BT-044-44 | Document delete leaves nodes | P0d D-3 | Coordinator E2E |
| BT-044-45 | Entity API 404 for existing graph node | P0d D-5 | Lookup E2E |
| BT-044-46 | Entity reconcile delete raw fails | P0d D-4 | Postgres reconcile test |
| BT-044-47 | Query enrichment missing nodes | P0d D-7 | Retrieval E2E |
| BT-044-48 | spec022 skip message wrong env var | P0b | Fix message + derive from DATABASE_URL |

---

## Rollback

| Action | Effect |
| ------ | ------ |
| Revert P0a only | Restores C-1 production bug |
| Revert + P1 fallback deletes | Deletes work; reads still broken |
| Schema rollback | Not required — app-only fix |

---

## Definition of done

### Code

- [ ] `cypher_query_bound` / `cypher_execute_bound` use bare `$1` + `String` bind + `sqlx::query`
- [ ] Zero `params_lit}'::agtype` in `cypher_exec.rs`
- [ ] Zero `raw_sql` in bound execute path

### Tests (all green with Postgres, no SKIP)

- [ ] `spec022_cypher_prepared_postgres` (integration + static)
- [ ] `spec044_compensation_postgres` (new)
- [ ] `postgres_integration::test_postgres_age_graph_crud`
- [ ] `storage_backend_contract` postgres
- [ ] `backend_e2e_contract` postgres
- [ ] `graph_isp_contract` postgres
- [ ] `./specs/044-upgrate-issue-study/e2e/run_upgrade_ingest_cypher_proof.sh`

### CI

- [ ] No `continue-on-error` on AGE Cypher contract steps
- [ ] **PG16 + PG17 + PG18** all green via `run_triple_track_cypher_proof.sh all`
- [ ] Reports in `e2e/reports/` for each profile

### Production

- [ ] Deploy v0.14.2
- [ ] `post_upgrade_health.sql` PASS
- [ ] Quarantine log rate → 0 for 24h
- [ ] Re-ingest previously failed documents

### Documentation

- [ ] [009-cross-reference-matrix.md](./009-cross-reference-matrix.md) FIX rows VERIFIED
- [ ] [006-similar-issues-audit.md](./006-similar-issues-audit.md) §13 checklist complete

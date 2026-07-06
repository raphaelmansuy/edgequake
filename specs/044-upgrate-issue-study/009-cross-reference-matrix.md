# SPEC-044 — Cross-Reference Matrix

| ID | Claim | Evidence | Status |
| -- | ----- | -------- | ------ |
| RC-044-1 | Post-upgrade ingest fails with merge error | Graylog 2026-07-06 `quantalogic-prd-edgequake` | **CONFIRMED** |
| RC-044-2 | Compensation quarantine on node C1236 | Graylog `compensation::quarantine` | **CONFIRMED** |
| RC-044-3 | AGE error: third argument must be a parameter | `cleanup_error` in log | **CONFIRMED** |
| RC-044-4 | `cypher_execute_bound` uses inline `'::agtype` | `cypher_exec.rs:78-80` | **CONFIRMED** |
| RC-044-5 | Regression introduced v0.14.0 #278 | git `c2dfe7ae` commit chain | **CONFIRMED** |
| RC-044-6 | v0.14.1 does not fix Cypher bind | CHANGELOG [0.14.1] | **CONFIRMED** |
| RC-044-7 | Relational schema OK post-migration | `edgequakeSchema.sql` | **CONFIRMED** |
| RC-044-8 | Merge hot path avoids bound Cypher | `nodes_ops.rs` batch/native | **CONFIRMED** |
| RC-044-9 | Later ingest succeeds (happy path) | Operator report | **CONFIRMED** |
| FIX-044-1 | P0: bare `$1` + text bind | `cypher_exec.rs` | ⬜ PLANNED |
| FIX-044-2 | spec022 postgres test required in CI | CI workflow | ⬜ PLANNED |
| FIX-044-3 | E2E `run_upgrade_ingest_cypher_proof.sh` | `e2e/` | ⬜ PLANNED |
| FIX-044-4 | P2: phase-scoped compensation | `merger/mod.rs` | ⬜ DEFERRED |
| FIX-044-5 | Remove CI continue-on-error AGE masks | `postgres-integration.yml` | ⬜ PLANNED |
| FIX-044-6 | Fix spec022 static `::agtype` assertion | `spec022_cypher_prepared_postgres.rs` | ⬜ PLANNED |
| FIX-044-7 | Document deletion graph cleanup | `orchestrator/deletion.rs` | ⬜ PLANNED |
| FIX-044-8 | Entity lookup `get_node` | `entity_graph_lookup.rs` | ⬜ PLANNED |
| SIM-044-1 | 6 broken graph ops inventory | `006-similar-issues-audit.md` §3 | ✅ DOCUMENTED |
| SIM-044-2 | 15+ downstream call sites | `006-similar-issues-audit.md` §4 | ✅ DOCUMENTED |
| SIM-044-3 | CI false green analysis | `006-similar-issues-audit.md` §8 | ✅ DOCUMENTED |
| SIM-044-4 | raw_sql regression vs pre-v0.14.0 | `006-similar-issues-audit.md` §2 | ✅ DOCUMENTED |
| E2E-044-1 | spec022 injection-safe CRUD | `spec022_cypher_prepared_postgres.rs` | ⬜ PENDING P0 |
| E2E-044-2 | Upgrade health SQL | `e2e/sql/post_upgrade_health.sql` | ⬜ PENDING |
| E2E-044-3 | Battle test runner | `e2e/run_upgrade_ingest_cypher_proof.sh` | ⬜ PENDING P0 |
| E2E-044-4 | Compensation postgres test | `spec044_compensation_postgres.rs` | ⬜ PLANNED |
| E2E-044-5 | Document delete graph test | `spec044_document_delete_graph_postgres.rs` | ⬜ PLANNED |
| E2E-044-6 | postgres_integration CRUD | `postgres_integration.rs` | ⬜ PENDING P0 |
| E2E-044-TT | Triple-track pg16+pg17+pg18 | `run_triple_track_cypher_proof.sh` | ⬜ PLANNED |
| E2E-044-TT-04 | Negative inline agtype all tiers | `cypher_param_contract.sql` | ⬜ PLANNED |
| E2E-044-TT-05 | PREPARE $1 delete all tiers | `cypher_param_contract.sql` | ⬜ PLANNED |

---

## Traceability

| Spec | Relationship |
| ---- | ------------ |
| **SPEC-021** | P-G5 `compensate_merge_failure`, quarantine semantics |
| **SPEC-022** | P-H7 parameterized Cypher — **regressed** in v0.14.0 |
| **SPEC-039** | Label bootstrap — ruled out for this incident |
| **SPEC-042** | Extension pins, triple-track images, version battle tests |
| **SPEC-016** | Edge-case register pattern for SC1 Cypher params |
| **CHANGELOG** | v0.14.0 (#278), v0.14.1 (#280 volume only) |

---

## Code map (expanded — see 006 for full)

```
cypher_exec.rs (C-1 root)
    ├── cypher_query_bound ──┬── nodes_ops: pg_has_node, pg_get_node
    │                        └── edges_ops: pg_has_edge, pg_get_edge
    └── cypher_execute_bound ┬── nodes_ops: pg_delete_node
                             └── edges_ops: pg_delete_edge

Downstream (symptom varies):
    compensation.rs ──────────────► delete_node/edge (quarantine)
    orchestrator/deletion.rs ─────► delete_node/edge (orphan on doc delete)
    entity_reconcile.rs ──────────► get/delete (admin reconcile fail)
    entity_graph_lookup.rs ───────► get_node (entity API 404)
    orchestrator/query_ops.rs ────► get_node (query enrichment gap)

Safe (Mode A — not in blast radius):
    merge hot path: get_nodes_batch, upsert_*_batch (native/UNWIND)
    scoped deletes: pg_delete_*_scoped (inline escape)
```

---

## External references

| Source | URL |
| ------ | --- |
| AGE Prepared Statements | https://age.apache.org/age-manual/master/advanced/prepared_statements.html |
| AGE issue #315 (literal vs param) | https://github.com/apache/age/issues/315 |
| v0.14.0 release #275/#278 | CHANGELOG [0.14.0] |

---

## Artifact index

| File | Role |
| ---- | ---- |
| `000-index.md` | Spec entry point |
| `001-five-whys.md` | Root cause chain |
| `002-first-principles.md` | AGE mode A/B/C |
| `003-code-is-law.md` | Source line evidence |
| `004-edge-cases-and-mitigations.md` | BT-044-01–48 + D-1–9 register |
| `005-risk-analysis.md` | R-044-01–18 matrix |
| `006-similar-issues-audit.md` | Exhaustive similar-issues + call graph |
| `007-triple-track-battle-test.md` | PG16/17/18 + AGE 1.6.0/1.7.0 official-doc matrix |
| `008-implementation-plan.md` | Phased P0a–P3 fix |
| `edgequakeSchema.sql` | Relational post-migration dump |
| `e2e/run_triple_track_cypher_proof.sh` | **Triple-track** battle test runner |
| `e2e/run_upgrade_ingest_cypher_proof.sh` | Single-host battle test |
| `e2e/sql/cypher_param_contract.sql` | AGE `$1` PREPARE/EXECUTE probes |
| `e2e/sql/post_upgrade_health.sql` | Operator SQL gates |
| `e2e/reports/` | Per-profile battle test reports |

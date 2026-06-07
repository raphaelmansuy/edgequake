# SPEC-006 — E2E Proof Index

**Spec ID:** `006-ensure-perf`  
**Last verified:** 2026-06-06 (P9 complete)  
**Method:** Code is law · First principles · Zero regression

---

## Proof Matrix

| # | Proof | Layer | Requirement |
|---|-------|-------|-------------|
| [001](001-p0-resource-budget-ssot-proof.md) | ResourceBudget SSOT | `edgequake-core` unit | BR-006-012 |
| [002](002-p0-graph-scan-ops-proof.md) | GraphScanOps push-down | `edgequake-storage` integration | TR-006-001 |
| [003](003-p0-list-entities-bounded-proof.md) | List entities bounded page | `edgequake-api` HTTP + storage | NFR-006-001 |
| [004](004-p0-graph-timeout-no-fallback-proof.md) | Graph timeout → 503 | `edgequake-api` unit | BR-006-014 |
| [005](005-p0-upload-limit-alignment-proof.md) | Body limit 50 MiB | `edgequake-api` + core constants | TR-006-019 |
| [006](006-static-allowlist-gate-proof.md) | `get_all_*` allowlist shrinks | shell script | BR-006-021 |
| [007](007-p1-delete-cascade-bounded-proof.md) | Delete cascade document-scoped | `edgequake-api` integration | NFR-006-002 |
| [008](008-p1-lineage-bounded-proof.md) | Lineage prefix/degree lookup | `edgequake-api` static + storage | TR-006-003 |
| [009](009-p2-relationship-lookup-proof.md) | Relationship id lookup | `edgequake-api` + storage | TR-006-004 |
| [010](010-p2-postgres-source-prefix-sql-proof.md) | Postgres SQL prefix push-down | `edgequake-storage` static | TR-006-005 |
| [011](011-p3-community-guard-proof.md) | Community detection admission guard | `edgequake-api` unit | NFR-006-003 |
| [012](012-p3-orchestrator-token-cap-proof.md) | Orchestrator 30k token SSOT | `edgequake-core` unit | RB-LLM-008 |
| [013](013-p3-source-ids-gin-index-proof.md) | source_ids GIN migration | shell + SQL | TR-006-006 |
| [014](014-p4-migration-production-safe-proof.md) | Migration 038 prod-safe package | shell + SQL | NFR-006-004 |
| [015](015-p4-edge-cases-battle-test-proof.md) | Edge cases + community lint | integration + shell | NFR-006-002 |
| [016](016-p5-bootstrap-size-aware-proof.md) | Bootstrap size-aware 038 + /ready | postgres e2e + unit | NFR-006-004 |
| [017](017-p6-community-seal-readiness-proof.md) | Community seal + readiness battle | unit + make target | NFR-006-003 |
| [018](018-p8-graph-materialization-guard-proof.md) | Graph materialization guard DRY | unit + static gate | NFR-006-001 |
| [019](019-p9-production-delivery-proof.md) | Production delivery (orchestrator + ops) | unit + static gate | UC-006-002 |

---

## Run All Proofs

```bash
make resource-proof
# or
./specifications/006-ensure-perf/e2e/run_resource_proof.sh
```

---

## Regression Gates (must stay green)

```bash
cargo test -p edgequake-api e2e_entities --quiet
cargo test -p edgequake-api e2e_document_deletion --quiet
cargo test -p edgequake-api integration_tests --quiet
```

---

## Cross-refs

- [008 Regression Contract](../008_regression_contract.md)
- [006 Architecture](../006_architecture_remediation.md)
- Allowlist: [support/get_all_allowlist.txt](../support/get_all_allowlist.txt)

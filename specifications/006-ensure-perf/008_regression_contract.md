# SPEC-006: Regression Contract — Zero Regression Gates

**Spec ID:** `006-ensure-perf`  
**Status:** Active (P0 proofs implemented)  
**Principle:** Code is law. If a test doesn't exist, the guarantee doesn't exist.

---

## 1. Contract Rules

**BR-006-020:** No PR merging SPEC-006 remediation unless all gates in §2 pass.

**BR-006-021:** New handler touching graph data must use `GraphScanOps` or appear on
**temporary allowlist** with linked violation ID and removal milestone.

**BR-006-022:** Changing any value in [004](004_resource_budget_catalog.md) requires
updating `resource_budget_defaults_test` in same PR.

---

## 2. CI Gate Matrix

| Gate ID | Command | Pass criteria |
|---------|---------|---------------|
| G-006-01 | `cargo test -p edgequake-api resource_safety` | All resource tests green |
| G-006-02 | `cargo test --workspace --lib` | No regressions |
| G-006-03 | `make resource-proof` | See §3 |
| G-006-04 | `cargo clippy --all-targets -D warnings` | Clean |
| G-006-05 | `scripts/spec006_no_get_all_api.sh` | Zero unallowlisted `get_all_*` in API |
| G-006-06 | `cargo test -p edgequake-api e2e_document_deletion` | Delete correctness preserved |
| G-006-07 | `cargo test -p edgequake-storage graph_scan` | Push-down parity tests |

---

## 3. `make resource-proof` (proposed Makefile target)

```makefile
# SPEC-006: resource safety proof suite
resource-proof:
	cargo test -p edgequake-api resource_safety -- --nocapture
	cargo test -p edgequake-storage graph_scan_ops -- --nocapture
	@./scripts/spec006_no_get_all_api.sh
	@./scripts/spec006_budget_catalog_sync.sh
	@echo "✓ SPEC-006 resource-proof passed"
```

---

## 4. Required Tests (implement with remediation)

### 4.1 `resource_safety_list_entities_bounded_memory`

**Covers:** V-006-001, NFR-006-001, UC-006-001

```rust
// SPEC-006: NFR-006-001
#[tokio::test]
async fn resource_safety_list_entities_bounded_memory() {
    // Setup: mock graph with 100_000 nodes (Postgres or test double)
    // Action: GET /entities?page=1&page_size=10
    // Assert: process RSS delta < 50 MB (or mock records 0 get_all_nodes calls)
    // Assert: response.total == 100_000, items.len() == 10
}
```

### 4.2 `resource_safety_delete_document_large_graph`

**Covers:** V-006-002, NFR-006-002, UC-006-002

```rust
// SPEC-006: NFR-006-002
#[tokio::test]
async fn resource_safety_delete_document_large_graph() {
    // Setup: 100k nodes, 5 entities sourced from target doc
    // Action: DELETE document
    // Assert: only 5 entities removed; no get_all_nodes invocation
    // Assert: shared entities retain other source_ids
}
```

### 4.3 `resource_safety_graph_timeout_no_full_load`

**Covers:** V-006-003, BR-006-014

```rust
// SPEC-006: BR-006-014
#[tokio::test]
async fn resource_safety_graph_timeout_no_full_load() {
    // Setup: mock graph storage that delays get_popular_nodes_with_degree
    // Action: GET /graph?max_nodes=100
    // Assert: 503 response, Retry-After header
    // Assert: get_all_nodes call count == 0
}
```

### 4.4 `resource_budget_defaults_test`

**Covers:** BR-006-012

```rust
// SPEC-006: BR-006-012
#[test]
fn resource_budget_defaults_match_catalog() {
    let budget = ResourceBudgetConfig::default();
    assert_eq!(budget.max_graph_nodes, 500);
    assert_eq!(budget.max_upload_bytes, 50 * 1024 * 1024);
    assert_eq!(budget.max_concurrent_extractions, 16);
    // ... full table from 004_resource_budget_catalog.md
}
```

### 4.5 `graph_scan_ops_parity_test`

**Covers:** LSP, TR-006-001

```rust
// SPEC-006: TR-006-001 — memory vs postgres return same page for same filter
#[tokio::test]
async fn graph_scan_ops_parity_memory_postgres() { /* ... */ }
```

### 4.6 `resource_safety_shared_entity_delete`

**Covers:** V-006-002 edge case — shared entity

```rust
// Document A + B share ALICE; delete A → ALICE remains with B sources
```

---

## 5. Static Analysis Scripts

### 5.1 `scripts/spec006_no_get_all_api.sh`

```bash
#!/usr/bin/env bash
# SPEC-006: G-006-05
set -euo pipefail
ALLOWLIST=".spec006/get_all_allowlist.txt"
MATCHES=$(rg 'get_all_nodes\(\)|get_all_edges\(\)' edgequake/crates/edgequake-api/src \
  | grep -v -f "$ALLOWLIST" || true)
if [[ -n "$MATCHES" ]]; then
  echo "SPEC-006 violation: unallowlisted get_all_* in API:"
  echo "$MATCHES"
  exit 1
fi
```

**Initial allowlist** (shrink each phase):

```text
# Phase 0 — all current violators listed with V-006-XXX comment
entity_crud.rs
relationships/list.rs
documents/delete/single.rs
graph_query/traversal.rs
lineage/queries.rs
# ... complete list from 003_codebase_audit.md §2.2
```

### 5.2 `scripts/spec006_budget_catalog_sync.sh`

Parses [004](004_resource_budget_catalog.md) table defaults and compares to
`ResourceBudgetConfig::default()` — fails on drift.

---

## 6. Observability Assertions (SPEC-018 integration)

When `EDGEQUAKE_OTEL_ENABLED=true`, resource guards must emit:

| Metric | Type | Labels |
|--------|------|--------|
| `edgequake_resource_admission_total` | counter | `operation`, `result=allow\|reject` |
| `edgequake_graph_materialize_active` | gauge | — |
| `edgequake_graph_scan_nodes_estimated` | histogram | `operation` |

Log on reject:

```json
{
  "level": "warn",
  "event": "resource_admission_rejected",
  "operation": "list_entities",
  "node_count": 200000,
  "threshold": 50000,
  "trace_id": "..."
}
```

---

## 7. Existing Tests That Must Not Break

| Test suite | Why |
|------------|-----|
| `e2e_document_deletion.rs` | Cascade correctness |
| `e2e_document_deletion_postgres.rs` | Postgres delete |
| `e2e_graph.rs` | Graph viewer |
| `integration_tests.rs` | API smoke |
| `observability_proof.rs` | SPEC-018 |
| `e2e_timeout_config.rs` | Pipeline env clamps |

---

## 8. Performance Baselines (no regression)

| Operation | Baseline (10k nodes) | Max regression |
|-----------|-------------------|----------------|
| `list_entities` page 1 | < 100 ms | +20% |
| `GET /graph?max_nodes=200` | < 2 s | +30% |
| `DELETE` 1 doc (100 entities) | < 5 s | +30% |
| Upload 1 MB text | < 60 s e2e | +50% (LLM variance) |

Store baselines in `edgequake/benches/resource_safety_bench.rs`.

---

## 9. PR Review Checklist

- [ ] No new `get_all_*` in `edgequake-api/src` (or allowlist updated with milestone)
- [ ] `ResourceBudget` used for any new numeric cap
- [ ] NFR/BR/TR ID in code comment
- [ ] Test added in §4 mapping table
- [ ] [004](004_resource_budget_catalog.md) updated if defaults change
- [ ] [005](005_violation_registry.md) violation marked resolved if applicable

---

## Cross-refs

- Violations: [005](005_violation_registry.md)
- Architecture: [006](006_architecture_remediation.md)
- ADR: [007](007_adr.md)
- Ops: [009](009_operator_runbook.md)

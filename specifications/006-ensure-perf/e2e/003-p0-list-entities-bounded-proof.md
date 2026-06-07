# SPEC-006 E2E Proof 003 — List Entities Bounded

**Covers:** NFR-006-001, UC-006-001, V-006-001  
**Tests:** `edgequake-api/tests/resource_safety_proof.rs`

| Test | What it proves |
|------|----------------|
| `resource_safety_list_entities_bounded_page` | Storage push-down: 2_500 nodes → page of 10 |
| `resource_safety_list_entities_http_pagination` | HTTP `GET /api/v1/graph/entities` returns `total=1200`, `items=25` |

## Setup

Seeds nodes with `tenant_id` + `workspace_id` via `GraphStorageMutateOps::upsert_node` (no `get_all_nodes`).

## Run

```bash
cargo test -p edgequake-api resource_safety_list_entities
```

## Code is law

- Handler: `handlers/entities/entity_crud.rs` — uses `list_nodes_filtered`
- **Removed:** `get_all_nodes()` from list path (allowlist updated)

# E2E Proof 015 — P4 Edge Cases Battle Test

**Requirement:** NFR-006-002, TR-006-004, NFR-006-003, BR-006-021  
**Layer:** `edgequake-api` integration + static lint  
**Status:** ✅ Verified 2026-06-06

---

## Claim

All documented edge cases for bounded graph ops are covered by automated tests; unguarded community detection cannot re-enter the API layer.

---

## Edge Case Matrix

| Edge case | Test | File |
|-----------|------|------|
| Legacy `source_id` pipe format | `resource_safety_cascade_legacy_source_id_pipe_format` | `resource_safety_proof.rs` |
| KV key ≠ document_id | `resource_safety_cascade_key_prefix_mismatch` | `resource_safety_proof.rs` |
| Tenant isolation on cascade | `resource_safety_cascade_tenant_isolation` | `resource_safety_proof.rs` |
| Relationship lookup by property `id` | `resource_safety_relationship_lookup_by_property_id` | `resource_safety_proof.rs` |
| Community guard reject (large) | `resource_safety_community_guard_rejects_large_graph` | `resource_safety_proof.rs` |
| Community guard allow (small) | `resource_safety_community_guard_allows_small_graph` | `resource_safety_proof.rs` |
| Threshold boundary (`==` allow) | `resource_safety_community_guard_threshold_boundary_allow` | `resource_safety_proof.rs` |
| Shared entity partial delete | `resource_safety_delete_cascade_bounded_scope` | `resource_safety_proof.rs` |
| Legacy pipe unit logic | `legacy_pipe_source_id_matches_document_scope` | `document_graph_cascade.rs` |
| Key prefix scope unit | `key_prefix_scope_includes_both_prefixes` | `document_graph_cascade.rs` |
| Unguarded community lint | `spec006_no_unguarded_community_api.sh` | `scripts/` |
| E2E shared entity smoke | `test_delete_preserves_shared_entities` | `e2e_document_deletion.rs` |

---

## Bug fixed during P4

**Legacy `source_id` shadowing:** Partial cascade updates now `remove("source_id")` when writing `source_ids`, preventing pipe-format fields from re-matching deleted document sources.

---

## Run

```bash
cargo test -p edgequake-api resource_safety --quiet
cargo test -p edgequake-api test_delete_preserves_shared_entities --quiet
./scripts/spec006_no_unguarded_community_api.sh
```

All included in `make resource-proof`.

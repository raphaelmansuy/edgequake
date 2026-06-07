# E2E Proof 011 — P3 Community Detection Guard

**Spec:** SPEC-006 P3  
**Requirement:** NFR-006-003, GraphOperation::CommunityDetection  
**Status:** ✅ Verified 2026-06-06

---

## First Principle

Full-graph algorithms (Louvain) must **fail at admission** when `node_count > threshold` — never silently load 200k nodes into RAM.

---

## Code Is Law

| Layer | Artifact |
|-------|----------|
| Guard SSOT | `edgequake-core/resource/guard.rs` — `CommunityDetection.requires_full_scan()` |
| API wrapper | `edgequake-api/services/graph_community.rs` — `detect_communities_guarded` |
| Error mapping | `ApiError::graph_too_large` → 503 + Retry-After: 60 |

---

## Automated Proof

```bash
cargo test -p edgequake-api graph_community --quiet
cargo test -p edgequake-api resource_safety_community_guard_rejects_large_graph --quiet
```

**Scenario:** 100 nodes, threshold 10 → admission reject before `detect_communities` runs.

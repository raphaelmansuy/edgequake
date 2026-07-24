# SPEC-084 — First Principles

> **Status**: Active  
> **Product pin**: EdgeQuake v0.21.0 (`19477c2d`)  
> **Cross-refs**: [README](README.md) · [Register](01-issue-register.md) · [Roadmap](03-implementation-roadmap.md)  
> **Inherits**: [SPEC-083 laws](../083-improvements/00-first-principles.md)

---

## 1. WHY this pack exists

Six production-facing GitHub issues (#331, #319, #318, #317, #316, #255) share a pattern: **symptoms look like UI or “missing index” bugs, but the root is an invariant mismatch** (wrong AGE table, counts vs page universe, global FIFO vs workspace expectation, COMPAT-GUARD vs gateway models).

Treating each as an isolated hotfix risks:

- Adding a parent `_ag_label_vertex` GIN that M070 correctly removed (AGE inheritance)
- “Fixing” Failed filter with enum casing while pagination remains wrong
- Calling Clear All (#309) a fix for selected bulk delete (#317)

This pack collapses each issue into laws, a single DRY fix, and e2e gates that prove the law.

---

## 2. Laws (SPEC-083 + SPEC-084)

Reuse SPEC-083 LAW-1…LAW-8. SPEC-084 adds:

```
  LAW-9   Index locality: query the table that owns the index (AGE child labels)
  LAW-10  Filter universe = count universe (status filter before pagination)
  LAW-11  Batch completeness is client+server contract (expected N, not “known so far”)
  LAW-12  Lifecycle at scale is O(batches), never O(docs)×SeqScan
  LAW-13  Fairness key matches product isolation key (workspace when claimed)
  LAW-14  Wire model identity: pass-through when operator already named provider/model
```

### ASCII: laws → surfaces

```
                 +------------------+
                 | LAW-3 SSOT       |
                 +--------+---------+
                          |
     +--------------------+--------------------+
     |                    |                    |
     v                    v                    v
 +--------+         +-----------+         +-----------+
 | LAW-9  |         | LAW-10/11 |         | LAW-14    |
 | AGE GIN|         | List/Query|         | LLM gate  |
 +---+----+         +-----+-----+         +-----+-----+
     |                    |                    |
     +----------+---------+----------+---------+
                |
                v
         +-------------+
         | LAW-8/12/13 |
         | E2E + pool  |
         +-------------+
```

---

## 3. SOLID mapping (how we implement)

| Letter | Meaning here | Shared primitives (DRY) |
|--------|--------------|-------------------------|
| **S** | One module owns one invariant | `source_lineage_sql` child-table scans; `ListDocumentsRequest.status`; batch track expected count; `WorkspaceFairness`; `WireModelId` |
| **O** | Extend via policy, not `if provider` sprawl | gateway / custom-base allowlist for slash models |
| **L** | Wipe-all and selected-delete share cascade discovery contracts | batched prefix discovery (SPEC-050 / SPEC-071) |
| **I** | Narrow APIs (`status` query param; batch delete by IDs) | no FE pretending `page_size=500` bypasses `MAX_PAGE_SIZE` |
| **D** | App depends on EXPLAIN/op-count proofs, not “it felt fast” | e2e matrix in `04-e2e-test-matrix.md` |

Anti-patterns banned:

- GIN on `_ag_label_vertex` while rows live on `"Node"`
- Client-only status filter after truncated page
- N× parallel single deletes for “bulk”
- Tenant-only fairness sold as workspace independence
- Rewriting gateway `provider/model` strings to `gpt-4.1-nano` silently

---

## 4. Locked architectural decisions

1. **#331**: Retarget `pg_node_counts_by_source_prefixes` to `"Node"`; keep M038 `idx_node_source_ids_gin`. **Reject** reporter’s parent-table GIN.
2. **#319**: Server-side `status` filter before pagination; `status_counts` remain global over the unfiltered (or date/pattern-filtered) workspace set.
3. **#318**: “GIA” = Query readiness (FEAT0007). Soft-gate/warn during active batch; fix track `is_complete` with expected batch size.
4. **#317**: Selected multi-delete gets a durable batch job (not wipe-all, not N× HTTP deletes).
5. **#316**: Workspace-aware claim/fairness within tenant; keep tenant caps and local LLM gates.
6. **#255**: Fix COMPAT-GUARD + `llm_full_id()`; do not invent factory auto-prefix that `edgequake-llm` 0.10.1 does not do. Absorb intent of PR #229.

---

## 5. Verification pin

| Field | Value |
|-------|-------|
| Tag | v0.21.0 |
| Commit | `19477c2d` |
| Audit date | 2026-07-24 |
| Code fixes in this pass | **None** (docs + GitHub comments only) |

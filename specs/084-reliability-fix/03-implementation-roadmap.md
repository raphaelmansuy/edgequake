# SPEC-084 — Implementation Roadmap

> Sprint 0→2 implemented 2026-07-24. Exit checkboxes below mark landed evidence.

---

## Sprint 0 — Stop the bleed (list + pool) ✅

| Issue | Exit criteria | Evidence |
|-------|---------------|----------|
| GH-331 | Count SQL JOINs `"Node"`; EXPLAIN uses `idx_node_source_ids_gin`; concurrent reconcile does not pool-timeout | `[x]` `e2e_issue331_*` (3) |
| GH-319 | `status` on list API; Failed filter returns rows when `status_counts.failed > 0` beyond page 100 | `[x]` `issue319_failed_filter_beyond_page_size` |

**PR shape**: preferably one PR for #331 (storage) + one PR for #319 (api+webui), or a single “Sprint 0 reliability” PR if small.

**Gates (landed)**:

```bash
cargo test -p edgequake-storage --features postgres --test e2e_issue331_node_counts_child_gin
cargo test -p edgequake-api --test spec027_e2e issue319
cargo clippy -p edgequake-storage -p edgequake-api --all-targets --features postgres -- -D warnings
# Deferred: pnpm exec playwright test issue319-failed-filter.spec.ts
```

---

## Sprint 1 — Lifecycle + gateways ✅

| Issue | Exit criteria | Evidence |
|-------|---------------|----------|
| GH-317 | Selected K-delete = one durable batch task; unselected remain | `[x]` `issue317_batch_delete_unselected_remain` |
| GH-255 | Slash models passthrough with custom base / allow flag; `llm_full_id` no double prefix; local-on-cloud still guarded | `[x]` `issue255_*` (3) |

**PR shape**: #317 (api+webui+tasks) separate from #255 (safety_limits + core) — different blast radius.

**Gates (landed)**:

```bash
cargo test -p edgequake-api --test e2e_document_deletion issue317
cargo test -p edgequake-api --lib issue255
# Deferred: pnpm exec playwright test issue317-bulk-delete.spec.ts
# Deferred: issue317_batch_delete_200_opcount
```

---

## Sprint 2 — Multi-workspace UX honesty ✅

| Issue | Exit criteria | Evidence |
|-------|---------------|----------|
| GH-318 | Track `expected_count`; Query banner/soft-gate during batch | `[x]` `issue318_track_not_complete_until_expected` + FE soft-gate |
| GH-316 | Workspace-fair claim + workspace lanes; two-workspace interleaved progress e2e | `[x]` `issue316_*` memory + PG claim |

**PR shape**: #318 (api+webui) then #316 (tasks) — scheduling change needs careful soak.

**Gates (landed)**:

```bash
cargo test -p edgequake-api --test spec027_e2e issue318
cargo test -p edgequake-tasks --test issue316_workspace_fair_claim
cargo test -p edgequake-tasks --features postgres --test postgres_claim_lease issue316
# Deferred: pnpm exec playwright test issue318-query-readiness.spec.ts
```

---

## Definition of Done (each issue)

1. Study edge cases EC-* addressed or explicitly deferred with reason  
2. Named e2e from [04-e2e-test-matrix.md](04-e2e-test-matrix.md) green in CI  
3. GitHub issue comment with commit SHA + test names  
4. Close issue only when audit flipped to FIXED in [01-issue-register.md](01-issue-register.md)

---

## Sequencing rationale

```
  Sprint0: #331 + #319
       |         (pool + list honesty — unblocks ops)
       v
  Sprint1: #317 + #255
       |         (delete scale + gateway prod)
       v
  Sprint2: #318 + #316
                 (UX readiness + multi-WS fairness)
```

#331 before #317 because selected delete at scale will re-hit bad SQL/pool if counts/reconcile still scan parent during UI refresh storms.

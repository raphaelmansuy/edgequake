# SPEC-087 — Finding Register

> **SSOT for status counts**  
> **Audit**: 2026-07-24 · **Wave 0 docs**: done · **Implementation**: Waves 1–3 landed 2026-07-24  
> **Status legend**: OPEN | PARTIAL | FIXED | WONTFIX  
> **Counts**: **7 FIXED / 1 PARTIAL / 0 OPEN**

DRY rule: one row per finding. Deep studies live under [`findings/`](findings/).

| ID | Finding | Sev | Surface | Wave | Study | Laws | Status |
|----|---------|-----|---------|------|-------|------|--------|
| `iss087_stats_n1` | Workspace stats embedding count uses per-doc KV N+1 under 4s timeout → 500 at scale | P0 | api+storage | 1 | [F-stats-n1-embedding.md](findings/F-stats-n1-embedding.md) | 31,33 | FIXED |
| `iss087_kv_count_trait` | `KVStorage` lacks `count_embedded_chunks_for_docs`; Postgres has no COUNT override | P0 | storage | 1 | [F-stats-n1-embedding.md](findings/F-stats-n1-embedding.md) | 33 | FIXED |
| `iss087_embedding_ssot` | Stats inspects KV `embedding` field not written by `chunk_kv_value`; ignores relational chunks COUNT | P1 | api+core | 1 | [F-stats-n1-embedding.md](findings/F-stats-n1-embedding.md) | 32 | FIXED |
| `iss087_anon_mint` | Per-browser `anon_*` INSERT on chat/conversation when PG present | P1 | api | 2 | [F-anon-user-mint.md](findings/F-anon-user-mint.md) | 29,30 | FIXED |
| `iss087_jwt_userid` | Auth-on path does not reliably bind `TenantContext.user_id` from JWT; FE localStorage drifts | P1 | api+webui | 2 | [F-anon-user-mint.md](findings/F-anon-user-mint.md) | 29,30 | FIXED |
| `iss087_admin_anon_filter` | Admin Users list has no anonymous/guest filter or label | P2 | api+webui | 2 | [F-anon-user-mint.md](findings/F-anon-user-mint.md) | 29 | FIXED |
| `iss087_allow_anonymous_flag` | Suggested `EDGEQUAKE_ALLOW_ANONYMOUS` missing; no strict deny mode | P2 | api | 2 | [F-anon-user-mint.md](findings/F-anon-user-mint.md) | 30 | FIXED |
| `iss087_anon_cleanup` | Existing orphan `anon_*` rows need reassign/delete playbook | P2 | ops | 3 | [F-anon-user-mint.md](findings/F-anon-user-mint.md) | 29,30 | PARTIAL |

---

## Wave summary

| Wave | Findings | Intent | Status |
|------|----------|--------|--------|
| 0 | (pack) | First principles, lenses, register, comments | **done** |
| 1 | `iss087_stats_n1`, `iss087_kv_count_trait`, `iss087_embedding_ssot` | O(1) stats aggregates + SSOT | **done** |
| 2 | `iss087_anon_mint`, `iss087_jwt_userid`, `iss087_admin_anon_filter`, `iss087_allow_anonymous_flag` | Shared guest + JWT bind + admin UX + flag | **done** |
| 3 | `iss087_anon_cleanup` + e2e + GitHub replies | Proof + operator cleanup playbook | **done** (cleanup ops PARTIAL — playbook ready, run on deploy) |

---

## Proof (2026-07-24)

| Suite | Result |
|-------|--------|
| `cargo test -p edgequake-storage --lib count_embedded` | pass |
| `cargo test -p edgequake-api --test e2e_dashboard_stats_issue81 test_spec087` | pass (scale 500 docs &lt; 4s) |
| `cargo test -p edgequake-api --test e2e_spec087_anonymous_guest` | pass (7 cases) |
| `spec027_identity_pg_rls_envelope_phase43` | pass (retargeted) |
| `cargo clippy -p edgequake-api --lib --features postgres -- -D warnings` | pass |
| `cargo clippy -p edgequake-storage --lib --features postgres -- -D warnings` | pass |
| `cargo clippy -p edgequake-auth --all-targets -- -D warnings` | pass |
| `cargo fmt --check -p edgequake-api -p edgequake-storage -p edgequake-auth` | pass |

---

## Inherited context (not re-opened as new IDs)

| Prior | Relevance |
|-------|-----------|
| [SPEC-027](../027-api-edgequake-audit/) | Documented anon bootstrap as FK safety — pin retargeted |
| [SPEC-024](../024-egdequake-audit/) | Chunk KV write path without embedding vectors |
| [Issue #81 e2e](../../edgequake/crates/edgequake-api/tests/e2e_dashboard_stats_issue81.rs) | Extended with SPEC-087 scale/SSOT cases |
| [SPEC-017](../017-dry-and-solid-audit/) | DRY bootstrap helper pattern |

---

## Status update rule

A finding moves to FIXED only when its verify IDs in [04-verification-matrix.md](04-verification-matrix.md) pass and the study file records proof date + command output summary.

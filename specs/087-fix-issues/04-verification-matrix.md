# SPEC-087 — Verification Matrix

> A finding is FIXED only when all linked verify IDs pass.

| Verify ID | Type | Finds | Command / check | Pass criteria |
|-----------|------|-------|-----------------|---------------|
| `iss087_v_count_trait` | unit/contract | `iss087_kv_count_trait` | `cargo test -p edgequake-storage` covering trait default + Postgres override (or source contract) | Method exists on `KVStorage`; PG impl present; empty docs → 0 |
| `iss087_v_stats_under_timeout` | api | `iss087_stats_n1` | Cold `GET /api/v1/workspaces/{id}/stats` after seed | HTTP 200 in &lt; 4s; no Internal timeout |
| `iss087_v_embedding_ssot` | contract | `iss087_embedding_ssot` | Compare stats `embedding_count` to SSOT (`chunks` COUNT or chunk-key count) | Equal within pinned definition |
| `iss087_e_scale_stats` | e2e | `iss087_stats_n1` | Seed ≥500 docs/chunk keys; cold stats | 200 + latency gate; see [e2e/README.md](e2e/README.md) |
| `iss087_v_shared_guest` | api | `iss087_anon_mint` | Auth off; two distinct `X-User-ID` chat creates | `users` anonymous/guest count = 1 per tenant |
| `iss087_e_incognito_no_growth` | e2e | `iss087_anon_mint` | Two browsers / clear localStorage + chat | No second `anon_*` row |
| `iss087_v_jwt_bind` | api | `iss087_jwt_userid` | Auth on; login; chat with mismatched header UUID | Conversation/`users` use JWT subject; no new anon |
| `iss087_e_auth_on_no_anon` | e2e | `iss087_jwt_userid` | Login flow + chat in WebUI | Zero new `@anonymous.local` rows |
| `iss087_v_admin_filter` | api+ui | `iss087_admin_anon_filter` | `GET` users default; toggle include | Default excludes guest/anon; include shows labeled Guest |
| `iss087_v_allow_anonymous_flag` | api | `iss087_allow_anonymous_flag` | `EDGEQUAKE_ALLOW_ANONYMOUS=false`, auth off, chat | 401/403; no INSERT |
| `iss087_v_cleanup_playbook` | ops | `iss087_anon_cleanup` | Run documented SQL against staging DB | Orphans reassigned/deleted; FK intact |

---

## Regression suite (must stay green)

| Suite | Why |
|-------|-----|
| `cargo test -p edgequake-api --test e2e_dashboard_stats_issue81` | Stats JSON shape / isolation |
| `cargo test -p edgequake-api --test spec027_api_contract` | Retargeted identity pin |
| Existing conversation / chat e2e with auth off | Shared guest still allows demo chat |
| WebUI Users management smoke | Filter/label does not break pagination |

---

## Manual prod-class check (optional but recommended)

| Check | Notes |
|-------|-------|
| ≥5k documents cold stats | Matches reporter environment class (#334) |
| Public auth-off instance user count overnight | Should not climb with crawler/browsers after Wave 2 |

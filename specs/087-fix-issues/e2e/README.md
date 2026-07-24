# SPEC-087 — E2E / Reproduction Matrix

> **Status**: Automation green 2026-07-24 (Waves 1–3).  
> **Product pin**: v0.21.1

---

## A. Issue #335 — Anonymous users

### A0. Manual reproduction (today — must fail closed after Wave 2)

| Step | Action | Observe today | Observe after fix |
|------|--------|---------------|-------------------|
| 1 | `EDGEQUAKE_AUTH_ENABLED=false`, start stack | healthy | healthy |
| 2 | Browser A: clear site data, open UI, create conversation / chat | works | works |
| 3 | Query `users` for `@anonymous.local` | ≥1 new `anon_*` | shared guest only |
| 4 | Browser B / incognito: chat again | **another** `anon_*` | **same** guest; count unchanged |
| 5 | Admin Users | spam rows | default list clean |

### A1. Automated cases

| ID | Case | Setup | Assert |
|----|------|-------|--------|
| `iss087_e_incognito_no_growth` | Two client UUIDs auth off | POST conversations with UUID_A then UUID_B | anonymous/guest user count = 1 |
| `iss087_e_auth_on_no_anon` | Login + chat | Auth enabled; JWT; optional wrong `X-User-ID` | no new `@anonymous.local`; ownership = JWT sub |
| `iss087_v_admin_filter` | Users list | Seed guest + real user | default list has real only; `include_anonymous=true` shows Guest |
| `iss087_v_allow_anonymous_flag` | Strict mode | `ALLOW_ANONYMOUS=false`, auth off | chat/create → 401/403; users count unchanged |

### A2. Suggested locations

| Kind | Target |
|------|--------|
| API e2e | `edgequake/crates/edgequake-api/tests/` new `e2e_spec087_anonymous_guest.rs` (or extend auth/conversation tests) |
| Playwright | `edgequake_webui/e2e/spec087-anonymous-users.spec.ts` (optional UI proof) |
| Contract | Update `spec027_api_contract.rs` pin |

---

## B. Issue #334 — Stats N+1

### B0. Manual reproduction (scale)

| Step | Action | Observe today | Observe after fix |
|------|--------|---------------|-------------------|
| 1 | Workspace with ≥5k docs (or ≥500 in CI harness) | — | — |
| 2 | Restart API / expire stats cache | cold | cold |
| 3 | `GET .../workspaces/{id}/stats` | 500 or &gt;4s | 200 ≪ 4s |
| 4 | Compare `embedding_count` to SSOT | often 0 / wrong | matches C-087-02 |

### B1. Automated cases

| ID | Case | Setup | Assert |
|----|------|-------|--------|
| `iss087_e_scale_stats` | Scale smoke | ≥500 docs with chunk keys (synthetic OK) | 200; duration &lt; 4s; no Internal timeout |
| `iss087_v_stats_under_timeout` | Handler budget | same | wall clock &lt; timeout |
| `iss087_v_count_trait` | Trait presence | compile/unit | default + PG override |
| `iss087_v_embedding_ssot` | Accuracy | known chunk count N | `embedding_count == N` (per pin) |

### B2. Suggested locations

| Kind | Target |
|------|--------|
| Extend | `e2e_dashboard_stats_issue81.rs` with scale module **or** new `e2e_spec087_stats_scale.rs` |
| Unit | `edgequake-storage` tests for Postgres COUNT / empty ids |
| Negative | Grep/contract: `stats.rs` must not contain per-doc `keys_with_prefix` loop for embeddings |

---

## C. Cross-cutting checklist

- [ ] Wave 1 verify IDs green  
- [ ] Wave 2 verify IDs green  
- [ ] Wave 3 cleanup playbook dry-run on staging  
- [ ] Comments posted on GitHub #334 and #335  
- [ ] Register updated to FIXED counts  

---

## D. Out of scope for these e2e

- Full 9k-doc production restore in CI (document as manual)  
- Temporal pipeline changes  
- SPEC-086 ingestion UX

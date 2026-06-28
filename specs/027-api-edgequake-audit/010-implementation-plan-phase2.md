# Implementation Plan — Phase 2–54 (AUTH + OIDC opt-in)

**Spec:** 027-api-edgequake-audit  
**Date:** 2026-06-28 (post phase 54)  
**Authority:** [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Scope (Phase 55 — COMPLETE — KV auth eliminated)

| P | Item | Status |
|---|------|--------|
| P0 | Delete `auth_kv_store`; add `AuthMemoryStore` | **DONE** |
| P0 | OIDC pending → memory (not KV) | **DONE** |
| P1 | Migration **065** + bootstrap | **DONE** |
| P1 | Contract + E2E green | **DONE** |

---

## Scope (Phase 54 — COMPLETE — Builtin OIDC opt-in)

| P | Item | Status |
|---|------|--------|
| P0 | `GET /api/v1/auth/oidc/login` + callback (PKCE) | **DONE** |
| P0 | `identity_storage` + `session_storage` on callback | **DONE** |
| P0 | OpenAPI + public path security for OIDC | **DONE** |
| P1 | Migration **064** + bootstrap reconcile | **DONE** |
| P1 | Contract `spec027_oauth2_oidc_builtin_wiring_phase54` | **DONE** |
| P1 | E2E `spec027_oidc_e2e` (7 tests, wiremock IdP) | **DONE** |
| P1 | OIDC lockout DRY (`login_lockout` service) | **DONE** |
| P1 | 004 / 009 doc re-assessment | **DONE** |

**Default deploy:** OIDC disabled (`oauth2_oidc_builtin: false`). External oauth2-proxy remains valid.

**Env:** `EDGEQUAKE_OIDC_ENABLED`, `EDGEQUAKE_OIDC_ISSUER_URL`, `EDGEQUAKE_OIDC_CLIENT_ID`, `EDGEQUAKE_OIDC_REDIRECT_URI`, optional secret + success redirect.

---

## Scope (Phase 53 — SUPERSEDED by phase 54)

Phase 53 locked zero OIDC routes. Phase 54 added opt-in builtin OIDC. Contract `spec027_oauth2_oidc_no_protocol_routes_phase53` is **ignored**.

---

## Scope (Phase 52 — COMPLETE — AUTH CLOSED)

| P | Item | Status |
|---|------|--------|
| P0 | session/api_keys → `session_storage` only | **DONE** |
| P0 | `auth_kv_store` service-layer only (2 modules) | **DONE** |
| P1 | Migration **063** + contract tests | **DONE** |

**No further auth/KV SSOT phases planned in SPEC-027.**

### Phase 52b (doc-only)

004 historical SEC-001 / Threat Model contradictions purged; 008 IMP-001 marked DONE (AC-4).

---

## Scope (Phase 51 — COMPLETE)

| P | Item | Status |
|---|------|--------|
| P0 | `handlers/auth/mod.rs` → `identity_storage` only | **DONE** |
| P1 | OpenAPI `ApiCapabilities` oauth example | **DONE** |
| P1 | Migration **062** (new SQL only) | **DONE** |

---

## Scope (Phase 50 — COMPLETE)

| P | Item | Status |
|---|------|--------|
| P0 | `user_management` isolated from `auth_kv_store` | **DONE** |
| P0 | `identity_storage::list_user_records` / `delete_user_record` | **DONE** |
| P1 | `pub(crate) mod auth_kv_store` | **DONE** |
| P1 | Remove dead KV index helpers | **DONE** |
| P1 | PG E2E OAuth `/health` + migration **061** (new SQL only) | **DONE** |

---

## Scope (Phase 49 — COMPLETE)

| P | Item | Status |
|---|------|--------|
| P0 | Document OAuth2/OIDC **not builtin** (`edgequake-auth` constants) | **DONE** |
| P1 | `/health` capabilities: mechanisms, oauth flag, KV harness, SSO pattern | **DONE** |
| P1 | Rename `mirror_user_record` → `persist_user_record_kv` | **DONE** |
| P1 | Migration **060** (new SQL only) + contract/E2E | **DONE** |

**No OAuth implementation** — operator honesty only. Past migration SQL files **not modified**.

---

## Scope (Phase 48 — COMPLETE)

| P | Item | Status |
|---|------|--------|
| P0 | DRY `pg_primary` / else branches in identity + session storage | **DONE** |
| P1 | `user_management` else-branch KV (delete, email index) | **DONE** |
| P1 | Remove dead `kv_mirror` revoke path on PG api-key revoke | **DONE** |
| P1 | Migration 059 + contract `spec027_pg_only_auth_branch_phase48` | **DONE** |

**No new auth behavior** — cleanup only.

---

## Scope (Phase 47 — COMPLETE)

| P | Item | Status |
|---|------|--------|
| P0 | Hard-disable KV mirror when PG pool (`IdentityPolicy`) | **DONE** |
| P1 | `/health` configured vs effective mirror flags | **DONE** |
| P1 | PG E2E proves env ignored | **DONE** |
| P1 | Migration 058 + contract tests | **DONE** |

---

## Scope (Phase 46 — COMPLETE)

KV mirror deprecated; `health_schema.rs`; migration 057.

---

## Scope (Phase 45 — COMPLETE) — IMP-026

`auth_kv_store.rs`; migration 056.

---

## Verification

```bash
cargo test -p edgequake-api --features postgres \
  --test spec027_api_contract --test spec027_e2e --test spec027_pg_auth_e2e
cargo clippy -p edgequake-api --features postgres -- -D warnings
```

**Last run:** 2026-06-28 — 117 contract + 1 ignored + 35 e2e + 6 pg ✅

**Status:** AUTH scope **CLOSED** per 009 phase 52.

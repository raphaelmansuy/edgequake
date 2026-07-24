# F-anon-user-mint — Silent per-browser anonymous users

> **Finding IDs**: `iss087_anon_mint`, `iss087_jwt_userid`, `iss087_admin_anon_filter`, `iss087_allow_anonymous_flag`, `iss087_anon_cleanup`  
> **Status**: FIXED (cleanup playbook PARTIAL — operator run on deploy)  
> **Wave**: 2 (cleanup docs Wave 3)  
> **Laws**: LAW-29, LAW-30  
> **Issue**: [#335](https://github.com/raphaelmansuy/edgequake/issues/335)  
> **Verify**: `iss087_v_shared_guest`, `iss087_v_jwt_bind`, `iss087_v_admin_filter`, `iss087_e_incognito_no_growth`, `iss087_e_auth_on_no_anon`

---

## 1. Symptom

When authentication is not enforced, every fresh browser (or cleared `localStorage`) that starts a chat or creates a conversation causes a new row in `users`:

| Field | Value |
|-------|--------|
| `username` | `anon_{first 8 hex of uuid}` |
| `email` | `{first 8}@anonymous.local` |
| `password_hash` | literal `'anonymous'` |
| `role` | `user` |
| `is_active` | `TRUE` |

Admin → Users fills with accounts the operator never created. On a public instance this grows without bound and looks like unauthorized registration.

Reporter environment: EdgeQuake **0.12.11** Docker; **still present on 0.21.1**.

---

## 2. Evidence (code is law)

| Path | Symbol / lines | Observation |
|------|----------------|-------------|
| `edgequake/crates/edgequake-api/src/services/identity_storage.rs` | `ensure_anonymous_user_in_postgres` ~L318–358 | INSERT with `anon_` / `@anonymous.local` / `'anonymous'` hash; `ON CONFLICT (user_id) DO NOTHING` |
| `edgequake/crates/edgequake-api/src/handlers/postgres_user_bootstrap.rs` | `ensure_postgres_user_exists` L12–38 | Always calls ensure when `pg_pool` present; **no env gate** |
| `edgequake/crates/edgequake-api/src/handlers/chat/completion.rs` | ~L86–87 | Caller |
| `edgequake/crates/edgequake-api/src/handlers/chat/streaming.rs` | ~L112–113 | Caller |
| `edgequake/crates/edgequake-api/src/handlers/conversations/crud.rs` | ~L119–120 | Create conversation caller |
| `edgequake_webui/src/lib/api/client-context.ts` | `getOrCreateUserId` | Random UUID in `localStorage` key `userId` |
| `edgequake_webui/src/lib/api/client.ts` | headers | Sets `X-User-ID` from `getOrCreateUserId()` |
| `edgequake/crates/edgequake-api/src/services/auth_bootstrap.rs` | `is_login_capable_password_hash` | Rejects `"anonymous"` — login blocked, list not filtered |
| `edgequake/crates/edgequake-api/tests/spec027_api_contract.rs` | ~L1072–1083 | **Pins** presence of `ensure_anonymous_user_in_postgres` (intentional FK path) |
| Makefile / quickstart | `DEV_AUTH_ENABLED ?= false` | Local default reproduces the bug |

`EDGEQUAKE_ALLOW_ANONYMOUS` — **does not exist** in codebase (grep Wave 0).

---

## 3. Root cause

Conversation FKs require a `users` row. SPEC-027 implemented that as “upsert whatever UUID the client sends.” The Web UI invents a new UUID per browser. With auth off (default for `make dev` / many Docker demos), every chat is an INSERT. Admin list has no anonymous filter. Result: unbounded operator-visible identity spam without login capability — a product and ops defect, not an account-takeover vector.

---

## 4. Fix (SOLID/DRY)

### Locked design (README)

1. **Shared per-tenant guest** when auth is off (deterministic UUID, stable `guest@anonymous.local` or equivalent, sentinel hash). Bootstrap ensures **that one row**, not the browser UUID.  
2. **Auth on**: bind `TenantContext.user_id` from JWT subject; FE syncs `localStorage.userId`; skip anon mint.  
3. **Admin**: default exclude/label anonymous sentinel hashes; `include_anonymous=true` opt-in.  
4. **`EDGEQUAKE_ALLOW_ANONYMOUS=false`**: refuse chat/conversation create when auth off instead of guest mint (strict mode).  
5. **Retarget** SPEC-027 contract pin to shared-guest helper name/behavior.  
6. **Cleanup playbook** for existing `anon_%` rows (reassign conversations to guest or delete).

### Shared primitives

- One identity helper (replace or reshape `ensure_anonymous_user_in_postgres` → `ensure_guest_or_real_user`).  
- One FE sync on login/logout.  
- One list filter predicate shared by API + UI labels.

### Non-goals

- Nullable `conversations.user_id`  
- Per-session anonymous-first mint (unbounded)  
- Removing open demo chat by default

---

## 5. Edge cases

| Case | Expected |
|------|----------|
| Auth off + incognito × N | Still **one** guest row per tenant |
| Auth on + stale localStorage UUID | JWT subject wins; no new anon row |
| Auth on + middleware missing user_id merge | Must be fixed — otherwise mint continues |
| Two UUIDs sharing first 8 hex chars | Current `anon_{8}` / email can collide on unique username/email — shared guest removes this class |
| Guest delete with conversations | FK: reassign or restrict delete |
| Multi-tenant | Guest is **per tenant_id**, not global |
| RLS / `PgIsolationScope` | Guest UUID must be valid membership scope |
| `EDGEQUAKE_ALLOW_ANONYMOUS=false` | 401/403; zero guest mint |
| Registration vs guest | Real users remain login-capable argon2/bcrypt hashes |
| List conversations with random X-User-ID | Must not mint (today list does not bootstrap — keep that) |

---

## 6. Proof

```text
Date: 2026-07-24
Commands:
  cargo test -p edgequake-api --test e2e_spec087_anonymous_guest
  cargo test -p edgequake-api --test spec027_api_contract spec027_identity_pg_rls_envelope_phase43
Result: pass — 7 guest/policy cases + retargeted SPEC-027 pin
```

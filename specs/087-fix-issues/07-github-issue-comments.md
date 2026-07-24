# SPEC-087 — GitHub Issue Comments (ready to post)

> Post in **Wave 3** after fixes land (or post analysis-only note in Wave 0 if desired).  
> Links: [#334](https://github.com/raphaelmansuy/edgequake/issues/334) · [#335](https://github.com/raphaelmansuy/edgequake/issues/335) · Spec pack: `specs/087-fix-issues/`

---

## Comment for #334 (stats N+1)

```markdown
### Maintainer analysis (SPEC-087) — confirmed on v0.21.1

**Reproduced / verified in current mainline.** Root cause is still live:

`try_kv_storage_stats` in `edgequake/crates/edgequake-api/src/handlers/workspaces/stats.rs` (≈L253–282) computes `embedding_count` with a **per-document** `keys_with_prefix` + `get_by_ids` loop under `STATS_FETCH_TIMEOUT = 4s` (L84). At ~5k–9k documents this is Θ(docs) round-trips and payload deserialization → cold-cache **timeout → 500**.

**Correction to the issue note:** on this tree, **neither** the `stats.rs` call-site fix **nor** the Postgres adapter override is present in 0.19.0/0.21.1. `git log -S count_embedded_chunks_for_docs` is empty. The trait method is also missing from `KVStorage` (`edgequake-storage/src/traits/kv.rs`). **All three pieces** are required (not trait-only).

**SSOT caveat:** current `chunk_kv_value` does **not** write an `embedding` field into chunk KV (SPEC-024 → vector/chunks path). A `jsonb_exists(value, 'embedding')` aggregate may return 0 even after the timeout is fixed. Prefer counting chunk keys or aligning with the existing O(1) relational path in `pg_get_workspace_stats` (`COUNT(*) FROM chunks WHERE workspace_id = $1`). Postgres override must use `self.table_name`, not a hardcoded `eq_eq_default_kv`.

**Tracking:** [specs/087-fix-issues](https://github.com/raphaelmansuy/edgequake/tree/main/specs/087-fix-issues) — Wave 1 implements trait default + Postgres COUNT + stats.rs call-site + scale e2e (`iss087_e_scale_stats`).
```

---

## Comment for #335 (anonymous users)

```markdown
### Maintainer analysis (SPEC-087) — confirmed on v0.21.1

**Reproduced / verified in current mainline.** Root cause matches the report:

1. Web UI `getOrCreateUserId()` stores a random UUID in `localStorage` and sends `X-User-ID`.
2. Chat completion/streaming and conversation **create** call `ensure_postgres_user_exists` → `ensure_anonymous_user_in_postgres` (`identity_storage.rs` ≈L318–358).
3. That INSERT creates `anon_{uuid[..8]}` / `{8}@anonymous.local` with `password_hash = 'anonymous'`.
4. There is **no** `EDGEQUAKE_ALLOW_ANONYMOUS` gate today. Admin Users does not filter these rows.
5. Login is blocked (`is_login_capable_password_hash` rejects `anonymous`) — not an account-takeover vector — but growth is unbounded and confusing on auth-off / demo deploys (`make dev` / quickstart often leave auth off).

**Intended fix (locked in SPEC-087):**
- Auth **off**: one **shared per-tenant guest** user (FK-safe, O(1) rows) — not one row per browser.
- Auth **on**: bind `user_id` from JWT subject; sync FE `localStorage`; never mint per-browser `anon_*`.
- Admin list: exclude/label guests by default; optional `include_anonymous`.
- Add `EDGEQUAKE_ALLOW_ANONYMOUS` for strict deny when auth is off.
- Cleanup playbook for existing orphans.

**Tracking:** [specs/087-fix-issues](https://github.com/raphaelmansuy/edgequake/tree/main/specs/087-fix-issues) — Wave 2 implementation; e2e `iss087_e_incognito_no_growth` / `iss087_e_auth_on_no_anon`.
```

---

## Optional short Wave-0 “analysis posted” note

If posting before code lands, prepend:

```markdown
> Analysis-only update: fix not merged yet. Spec pack Wave 0 documents root cause, lenses, and implementation waves.
```

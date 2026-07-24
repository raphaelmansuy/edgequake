# LENS — Full Stack

> **Laws**: LAW-29, LAW-30, LAW-31, LAW-32  
> **Findings**: all `iss087_*`

---

## 1. Question

Where does each bug cross FE ↔ API ↔ DB, and which seam owns the fix?

---

## 2. Identity stack (#335)

```
WebUI getOrCreateUserId()
  → localStorage["userId"] = random UUID
  → API client sets X-User-ID
  → TenantContext.user_id
  → ensure_postgres_user_exists
  → users INSERT anon_*
  → Admin Users list (no filter)
```

| Layer | Break | Fix owner |
|-------|-------|-----------|
| FE | Random UUID ≠ authenticated subject | Sync on login; prefer JWT user id |
| Middleware | Auth on may not set `user_id` from JWT | `apply_authenticated_context` must bind subject |
| Bootstrap | Always mints client UUID | Shared guest / real user only |
| Admin API/UI | Lists all rows | Default exclude anonymous |

### Auth-off vs auth-on

| Mode | Client identity | Server identity |
|------|-----------------|-----------------|
| Auth off (dev) | May keep local UUID for UX, but **server** maps to shared guest | Shared guest UUID |
| Auth on | `localStorage.userId` = JWT user id | JWT subject; no anon mint |

Do not leave a split brain where FE thinks it is user A and DB conversations attach to guest/anon B without an explicit product rule.

---

## 3. Stats stack (#334)

```
Dashboard
  → GET /api/v1/workspaces/{id}/stats
  → cache miss
  → 4s timeout wrapper
  → try_kv_storage_stats
  → N+1 KV scan
  → timeout → 500 or stale
```

| Layer | Break | Fix owner |
|-------|-------|-----------|
| Handler | Inline loop | Call trait / relational COUNT |
| Trait | Missing method | Default + Postgres override |
| Write path | No `embedding` in KV | Align SSOT with chunks/vector |
| Core service | Already O(1) | Reuse or align HTTP path |
| Tests | Only 50 docs | Add scale smoke |

---

## 4. Contract tests to retarget

- `spec027_api_contract.rs` currently asserts source contains `ensure_anonymous_user_in_postgres` — after Wave 2, pin shared-guest helper instead.  
- Dashboard e2e must assert latency/SSOT, not only JSON shape.

---

## 5. Acceptance for this lens

- [ ] One documented identity flow for auth on/off  
- [ ] One stats aggregation path (no dual slow/fast SSOTs left live)  
- [ ] FE + API + DB e2e covering both issues

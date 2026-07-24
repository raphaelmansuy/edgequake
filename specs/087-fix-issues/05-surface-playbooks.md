# SPEC-087 — Surface Playbooks

> Commands to reproduce, inspect, and (after Waves 1–2) verify fixes.

---

## 0. Common setup

```bash
# From repo root
make postgres-start
# Auth OFF reproduces #335 (Makefile default for dev)
export EDGEQUAKE_AUTH_ENABLED=false
export EDGEQUAKE_DEV_MODE=true
make backend-bg   # or make dev-bg
curl -s http://localhost:8080/health | python3 -m json.tool
```

---

## 1. Reproduce #335 — anonymous user mint

```bash
# Browser A: open UI, start chat (or API below with UUID_A)
UUID_A=$(uuidgen | tr '[:upper:]' '[:lower:]')
UUID_B=$(uuidgen | tr '[:upper:]' '[:lower:]')

# Adjust tenant/workspace headers to match your local TenantContext defaults if required.
curl -s -X POST http://localhost:8080/api/v1/conversations \
  -H "Content-Type: application/json" \
  -H "X-User-ID: $UUID_A" \
  -d '{"title":"repro-a"}'

curl -s -X POST http://localhost:8080/api/v1/conversations \
  -H "Content-Type: application/json" \
  -H "X-User-ID: $UUID_B" \
  -d '{"title":"repro-b"}'

# Inspect (psql against edgequake DB)
docker exec -it edgequake-postgres \
  psql -U edgequake -d edgequake \
  -c "SELECT username, email, password_hash, created_at FROM users WHERE email LIKE '%@anonymous.local' ORDER BY created_at DESC LIMIT 20;"
```

**Expect today:** two (or more) `anon_*` rows.  
**Expect after Wave 2:** one shared guest row regardless of UUID_A/B.

### Admin UI path

1. Auth off → chat in normal + incognito.  
2. Enable auth / login as admin → Settings → User Management.  
3. Observe `anon_*` spam (today) vs filtered list (after Wave 2).

---

## 2. Reproduce #334 — stats timeout

Full 5k-doc ingest is heavy; minimal signal:

```bash
# After workspace has many documents (or synthetic chunk keys in test harness):
WS_ID=<workspace-uuid>

# Cold path: restart API or wait out cache TTL (60s) then:
time curl -s -o /tmp/stats.json -w "%{http_code} %{time_total}\n" \
  "http://localhost:8080/api/v1/workspaces/${WS_ID}/stats"

cat /tmp/stats.json | python3 -m json.tool | head
```

**Expect today at scale:** HTTP 500 (cold) or `stale: true` (warm after prior success) with long `time_total`.  
**Expect after Wave 1:** HTTP 200, `time_total` ≪ 4s, accurate `embedding_count`.

### Code-level confirmation (no scale needed)

```bash
rg -n "count_embedded_chunks_for_docs" edgequake/crates/edgequake-storage
rg -n "for doc_id in &workspace_doc_ids" edgequake/crates/edgequake-api/src/handlers/workspaces/stats.rs
# Today: first command empty; second matches the N+1 loop
```

---

## 3. Post-fix verification snippets

```bash
# Wave 1
cargo test -p edgequake-storage --lib
cargo test -p edgequake-api --test e2e_dashboard_stats_issue81
# plus iss087 scale test when added

# Wave 2
cargo test -p edgequake-api --test spec027_api_contract
# Auth-off double chat → guest count == 1
# Auth-on mismatched X-User-ID → no anon insert
```

---

## 4. Orphan cleanup (Wave 3 / `iss087_anon_cleanup`)

After deploying Waves 1–2, collapse legacy per-browser `anon_*` rows onto the shared guest.

```sql
-- 1) Inventory
SELECT user_id, tenant_id, username, email, created_at
FROM users
WHERE email LIKE '%@anonymous.local'
   OR password_hash = 'anonymous'
   OR username LIKE 'anon_%'
ORDER BY tenant_id, created_at;

-- 2) Per tenant: ensure guest exists (app does this on next auth-off chat),
--    or insert guest via shared_guest_user_id(tenant) + guest@anonymous.local

-- 3) Reassign conversations (repeat per tenant; set :guest and :orphans)
-- BEGIN;
-- UPDATE conversations SET user_id = :guest WHERE user_id = ANY(:orphans);
-- DELETE FROM users WHERE user_id = ANY(:orphans)
--   AND user_id <> :guest
--   AND (email LIKE '%@anonymous.local' OR password_hash = 'anonymous');
-- COMMIT;
```

Always:

1. Inventory `@anonymous.local` / `password_hash = 'anonymous'`.  
2. Ensure shared guest exists (chat once with auth off, or manual INSERT).  
3. Reassign `conversations.user_id` (and any other FKs).  
4. Delete orphan users in a transaction.  
5. Re-check admin Users + chat.

---

## 5. Log locations

| Service | Log |
|---------|-----|
| Backend | `/tmp/edgequake-backend.log` |
| Frontend | `/tmp/edgequake-frontend.log` |

```bash
grep -i "anonymous user ensure\|Workspace stats" /tmp/edgequake-backend.log | tail -40
```

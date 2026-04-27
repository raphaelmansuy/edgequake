# Fix: Issues #189 and #192 — Frontend Cannot Reach API

> **Status:** FIXED  
> **Severity:** CRITICAL — WebUI unusable for any non-default deployment  
> **Issues:** [#189 — Unable to customize port](https://github.com/raphaelmansuy/edgequake/issues/189), [#192 — Connection Error](https://github.com/raphaelmansuy/edgequake/issues/192)

---

## Documents

| File | Purpose |
|------|---------|
| `README.md` (this file) | Index, executive summary, fix instructions |
| `root_cause_analysis.md` | First-principles analysis, architecture diagrams |
| `proof.md` | Logic proof + test results |

---

## One-Sentence Root Cause

The `frontend` Docker service had no `EDGEQUAKE_API_URL` environment variable, so the
WebUI always used a build-time-baked `http://localhost:8080` that cannot be changed at
runtime — breaking any non-default port or remote-access deployment.

---

## Architecture (Before Fix)

```
  User's browser
       |
       | GET http://HOST:FRONTEND_PORT/
       v
  +---------------------+    docker-compose frontend service
  | Next.js  :3000      |    environment:
  |                     |      NODE_ENV: production
  | getRuntimeConfig()  |      (nothing else!)
  |   EDGEQUAKE_API_URL |
  |    = undefined      |    Result: apiUrl = "" (empty)
  |   NEXT_PUBLIC_API_URL
  |    = "http://localhost:8080"   <- BUILD-TIME baked, immutable
  |                     |
  +---------------------+
       |                    |
  REST /api/v1              WebSocket
  (relative path)           ws://localhost:8080 (hardcoded fallback)
       |                         |
       v                         v
  PORT 3000 (Next.js)       Works only if browser IS the Docker host
  No proxy -> 404/FAIL      Fails for any remote browser
```

---

## Architecture (After Fix)

```
  User's browser
       |
       | GET http://HOST:FRONTEND_PORT/
       v
  +---------------------+    docker-compose frontend service
  | Next.js  :3000      |    environment:
  |                     |      NODE_ENV: production
  | getRuntimeConfig()  |      EDGEQUAKE_API_URL: http://localhost:${EDGEQUAKE_PORT:-8080}
  |   EDGEQUAKE_API_URL |
  |    = "http://HOST:PORT"    <- Runtime env var, set at container start
  |                     |
  | Injects into page:  |
  | window.__EDGEQUAKE_RUNTIME_CONFIG__ = { apiUrl: "http://HOST:PORT" }
  +---------------------+
       |                    |
  REST /api/v1              WebSocket
  http://HOST:PORT/api/v1   ws://HOST:PORT/ws/pipeline/progress
       |                         |
       v                         v
  API container :8080       API container :8080
  WORKS                     WORKS
```

---

## Files Changed

| File | Change |
|------|--------|
| `edgequake_webui/src/lib/runtime-config.ts` | Read `EDGEQUAKE_API_URL` (runtime) before `NEXT_PUBLIC_API_URL` (build-time) |
| `edgequake_webui/src/lib/websocket/websocket-manager.ts` | Replace hardcoded `ws://localhost:8080` with relative-origin fallback |
| `docker-compose.quickstart.yml` | Add `EDGEQUAKE_API_URL` to frontend service |

---

## Usage After Fix

```bash
# Default (no change needed)
docker compose -f docker-compose.quickstart.yml up -d
# Frontend → http://localhost:3000  API → http://localhost:8080

# Custom API port (Issue #189)
EDGEQUAKE_PORT=8081 docker compose -f docker-compose.quickstart.yml up -d
# Frontend → http://localhost:3000  API → http://localhost:8081  WORKS

# Remote/server access (Issue #192 use case)
EDGEQUAKE_API_URL=http://192.168.1.10:8080 \
  docker compose -f docker-compose.quickstart.yml up -d
# Users on other machines can reach the API
```

# Root Cause Analysis: Issues #189 & #192

> **Cross-refs:** [README.md](README.md), [proof.md](proof.md)

---

## First Principles: How the Frontend Knows Where the API Is

```
  PRINCIPLE: The browser's JS bundle must know the absolute URL of the API.
  
  In a Docker deployment, the bundle is pre-built. The URL cannot come from
  build-time env vars IF the URL changes between deployments.
  
  There are only two correct mechanisms:
  
  A) Server-side injection at request time:
     Server component reads a RUNTIME env var → injects into HTML
     Browser reads from injected script → correct URL every time
  
  B) Relative URL + reverse proxy:
     Browser calls /api/v1 → same origin → proxy forwards to API
     Requires nginx/traefik configuration not present here
  
  The codebase chose (A): window.__EDGEQUAKE_RUNTIME_CONFIG__ injection.
  But the docker-compose file never set the required env var.
  Result: apiUrl = "" → falls back to broken behavior.
```

---

## The NEXT_PUBLIC_* Build-Time Trap

```
  Next.js compilation:

  SOURCE (runtime-config.ts):
    process.env.NEXT_PUBLIC_API_URL

  COMPILED OUTPUT:
    "http://localhost:8080"   <- literal string, replaced at compile time

  This is deterministic and intentional by Next.js design:
  NEXT_PUBLIC_* vars are safe to expose to the browser BECAUSE they are
  inlined — no server process is needed at runtime to read them.

  CONSEQUENCE: The Docker image always has http://localhost:8080 hardcoded
  in its JS bundle. No amount of env vars at container start can change it
  for the client-side bundle.

  EXCEPTION: Server Components (layout.tsx) run at request time on the
  Node.js server. They CAN read process.env at runtime — but only for
  variables WITHOUT the NEXT_PUBLIC_ prefix (those are inlined too).
```

---

## Resolution Chain (Old vs New)

```
  OLD CHAIN (broken):
  ==================
  docker-compose: frontend has no EDGEQUAKE_API_URL
       |
       v
  layout.tsx (server component) calls getRuntimeConfig()
       |
       v
  getRuntimeConfig():
    browserConfig?.apiUrl    = undefined (SSR, no window)
    process.env.NEXT_PUBLIC_API_URL = "http://localhost:8080"  (BUILD-TIME baked)
       |
       v
  Injects: window.__EDGEQUAKE_RUNTIME_CONFIG__ = { apiUrl: "http://localhost:8080" }
       |
       v
  Browser reads apiUrl = "http://localhost:8080"
       |
       +--- REST calls → http://localhost:8080/api/v1
       |      Works ONLY if user's browser is on Docker host at default port
       |
       +--- WebSocket fallback (when baseUrl empty) → ws://localhost:8080
              Was never empty in practice, but hardcoded port 8080 regardless

  NEW CHAIN (fixed):
  ==================
  docker-compose: EDGEQUAKE_API_URL=http://localhost:${EDGEQUAKE_PORT:-8080}
       |
       v
  layout.tsx (server component) calls getRuntimeConfig()
       |
       v
  getRuntimeConfig():
    browserConfig?.apiUrl       = undefined (SSR, no window)
    process.env.EDGEQUAKE_API_URL = "http://localhost:8081"  (RUNTIME, from docker-compose)
       |
       v
  Injects: window.__EDGEQUAKE_RUNTIME_CONFIG__ = { apiUrl: "http://localhost:8081" }
       |
       v
  Browser reads apiUrl = "http://localhost:8081"
       |
       +--- REST calls → http://localhost:8081/api/v1  CORRECT
       |
       +--- WebSocket → ws://localhost:8081/ws/pipeline/progress  CORRECT
```

---

## Issue #189 Specific Analysis

```
  User command:
    EDGEQUAKE_PORT=8081 FRONTEND_PORT=3001 docker compose ... up -d

  docker-compose port binding:
    api:      host 8081 → container 8080  (EDGEQUAKE_PORT changes host side only)
    frontend: host 3001 → container 3000

  User opens: http://HOST:3001
  Browser downloads JS bundle with apiUrl = "http://localhost:8080"  (build-time baked)
  Browser calls: http://localhost:8080/api/v1  <- WRONG (should be 8081)
  Result: "API status disconnected"

  WITH FIX:
  docker-compose injects: EDGEQUAKE_API_URL=http://localhost:8081
  Browser receives injected config: { apiUrl: "http://localhost:8081" }
  Browser calls: http://localhost:8081/api/v1  <- CORRECT
```

---

## Issue #192 Specific Analysis

```
  User setup:
    - Ollama on 10.0.0.242:11434 (remote, systemd)
    - Docker on local machine
    - Accessing WebUI from the same machine (curl localhost:8080/health works)

  Observed: "Connection Error: Unable to connect to the server"
  Health check: passes (API is running)

  Why the disconnect?
  curl localhost:8080/health = direct TCP from shell → works
  Browser JS → uses apiUrl = "" (no env var set)
  
  When apiUrl = "":
    getRuntimeApiBaseUrl() = "/api/v1"  (relative path)
    Browser calls: http://HOST:3000/api/v1
    Next.js server has no /api/v1 route and no proxy config
    → 404 / connection refused → "Connection Error"
  
  AND:
    WebSocket: baseUrl="" is falsy → falls through to window.location.host path
    ws://HOST:3000/ws/pipeline/progress → Next.js has no WebSocket handler
    → Connection Error on status panel

  WITH FIX:
  EDGEQUAKE_API_URL defaults to http://localhost:8080
  apiUrl = "http://localhost:8080"
  All requests correctly target the API container
```

---

## The Hardcoded ws://localhost:8080 Defect

```
  ORIGINAL CODE (websocket-manager.ts):

  function getWebSocketUrl(): string {
    const baseUrl = getRuntimeServerBaseUrl();
    if (baseUrl) { ... use baseUrl ... }
    if (typeof window !== "undefined") { ... use window.location.host ... }
    return "ws://localhost:8080/ws/pipeline/progress";  // <- NEVER CORRECT
  }

  The last line is the "SSR fallback" — runs when:
    - window is undefined (server-side rendering)
    - AND baseUrl is empty
  
  In SSR context, this string is used to construct the initial WebSocket URL
  that gets hydrated into the client. If the client then can't resolve it
  (different host, different port), the WebSocket fails immediately.

  FIX: Replace with "/ws/pipeline/progress" (relative, protocol-agnostic).
  In SSR context this is never opened as a real WebSocket anyway.
  The client-side path (window check above) handles the real connection.
```

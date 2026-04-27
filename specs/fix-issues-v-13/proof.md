# Proof: Issues #189 & #192 Fixes

> **Cross-refs:** [README.md](README.md), [root_cause_analysis.md](root_cause_analysis.md)

---

## Proof A — Logic Simulation (Node.js)

Simulates `getRuntimeConfig()` and `getRuntimeApiBaseUrl()` for all scenarios.

```
=== BEFORE FIX ===
Issue #189 - injected apiUrl (build-baked): http://localhost:8080
  User changed EDGEQUAKE_PORT=8081, but browser still calls: http://localhost:8080 -> WRONG PORT

Issue #192 - apiUrl empty: true
  getRuntimeApiBaseUrl() returns: /api/v1
  /api/v1 -> relative path -> Next.js server (port 3000) -> no proxy -> FAIL
  WebSocket -> ws://localhost:8080 (hardcoded) -> if browser not on docker host -> FAIL

=== AFTER FIX ===
Issue #189 FIXED - injected apiUrl: http://localhost:8081  <- runtime, follows EDGEQUAKE_PORT
Issue #192 FIXED - injected apiUrl: http://localhost:8080
  getRuntimeApiBaseUrl(): http://localhost:8080/api/v1  <- correct API target
  WebSocket -> uses baseUrl -> ws://localhost:8080/ws/pipeline/progress  <- correct
  Browser window override still works: http://custom.example.com:9090  <- window wins

ALL SCENARIOS PASS
```

---

## Proof B — TypeScript Compile Check

```bash
cd edgequake_webui
npx tsc --noEmit --strict
# Exit: 0  (no type errors)
```

---

## Proof C — Test Suite

```bash
cd edgequake_webui
npx vitest run src/lib/__tests__/websocket-client.test.ts --reporter=verbose

# Result:
# Test Files  1 passed (1)
#       Tests  31 passed (31)
#    Duration  96ms
```

All 31 WebSocket client tests pass with no modifications.

---

## Proof D — Docker Compose Variable Resolution

```bash
# Issue #189: custom port scenario
EDGEQUAKE_PORT=8081 docker compose -f docker-compose.quickstart.yml config 2>/dev/null \
  | grep -A2 "EDGEQUAKE_API_URL"

# Expected output:
#   EDGEQUAKE_API_URL: http://localhost:8081

# Default port scenario
docker compose -f docker-compose.quickstart.yml config 2>/dev/null \
  | grep -A2 "EDGEQUAKE_API_URL"

# Expected output:
#   EDGEQUAKE_API_URL: http://localhost:8080
```

---

## Proof E — Resolution Priority Table

| Scenario | `browserConfig?.apiUrl` | `EDGEQUAKE_API_URL` | `NEXT_PUBLIC_API_URL` | Result |
|----------|------------------------|--------------------|-----------------------|--------|
| Default Docker | undefined | `http://localhost:8080` | `http://localhost:8080` | `http://localhost:8080` ✓ |
| Custom port 8081 | undefined | `http://localhost:8081` | `http://localhost:8080` | `http://localhost:8081` ✓ |
| Remote access | undefined | `http://192.168.1.10:8080` | `http://localhost:8080` | `http://192.168.1.10:8080` ✓ |
| Browser override | `http://custom:9090` | anything | anything | `http://custom:9090` ✓ |
| Local dev (.env.local) | undefined | undefined | `http://localhost:8080` | `http://localhost:8080` ✓ |

Priority: `window.__EDGEQUAKE_RUNTIME_CONFIG__` > `EDGEQUAKE_API_URL` (runtime) > `NEXT_PUBLIC_API_URL` (build-time) > `""`

---

## Proof F — WebSocket Fallback Fix

```
  Before (websocket-manager.ts SSR fallback):
    return "ws://localhost:8080/ws/pipeline/progress";
    
    Problem: hardcodes port 8080. If apiUrl is empty and window is undefined
    (SSR), the hydrated URL sent to the browser contained this string.
    On any non-default-port setup, WebSocket connection fails immediately.

  After:
    return "/ws/pipeline/progress";
    
    Relative URL. In SSR context this is never used to open a real socket.
    The client-side branch (window check) runs in browsers and correctly
    uses the injected apiUrl from window.__EDGEQUAKE_RUNTIME_CONFIG__.
```

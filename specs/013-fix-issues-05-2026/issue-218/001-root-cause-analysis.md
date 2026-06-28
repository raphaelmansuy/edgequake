# Issue #218 — Root Cause Analysis

**GitHub:** [#218](https://github.com/raphaelmansuy/edgequake/issues/218)  
**Cross-ref:** [issue-218/002-fix-specification.md](002-fix-specification.md), [../implementation/issue-218-runtime-config.spec.ts](../implementation/issue-218-runtime-config.spec.ts)

## Symptom (fact)

Prebuilt frontend image ignores `EDGEQUAKE_API_URL`, `NEXT_PUBLIC_AUTH_ENABLED`, `NEXT_PUBLIC_DISABLE_DEMO_LOGIN` at runtime; HTML injects `http://localhost:8080`.

## 5 WHY

| # | Why | Evidence |
|---|-----|----------|
| 1 | Why does the browser call localhost:8080? | `window.__EDGEQUAKE_RUNTIME_CONFIG__.apiUrl` is `http://localhost:8080` in served HTML |
| 2 | Why is the injected config wrong? | `getRuntimeConfig()` in `layout.tsx` ran at **build/static** time with empty `EDGEQUAKE_API_URL` |
| 3 | Why at build time? | Root layout had no `dynamic = 'force-dynamic'` — Next.js statically optimized the layout |
| 4 | Why was PR #193 insufficient? | Runtime injection code exists but never re-executes per request without dynamic layout |
| 5 | Why does this break ECS? | Container env vars are set at **runtime**; static shell cannot read them |

## Proof (code)

```33:42:edgequake_webui/src/app/layout.tsx
  const runtimeConfig = getRuntimeConfig();
  // ... injects window.__EDGEQUAKE_RUNTIME_CONFIG__
```

`getRuntimeConfig()` reads `process.env.EDGEQUAKE_API_URL` server-side — only correct when layout renders per request.

## Fix summary

Add `export const dynamic = 'force-dynamic'` to root layout (one line, restores PR #193 intent).

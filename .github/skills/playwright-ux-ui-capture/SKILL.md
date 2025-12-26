---
name: playwright-ux-ui-capture
description: Capture EdgeQuake WebUI routes with Playwright and write artifacts immediately (screenshots + per-page request JSON + capture index). Use when adding/updating Playwright E2E capture specs or when asked to automate UI screenshot collection.
license: Proprietary (repository internal)
compatibility: Requires Node.js, Next.js dev server, and @playwright/test in edgequake_webui; runs via npm scripts.
metadata:
  repo: raphaelmansuy/edgequake
  area: e2e
---

# Playwright UX/UI Capture (Artifact-first)

## When to use
Use this skill when implementing or modifying Playwright E2E to:
- capture screenshots for a set of routes
- emit per-route artifacts immediately to disk
- keep the process robust and deterministic

## Repo conventions
- Playwright config: `edgequake_webui/playwright.config.ts`
- Tests live in: `edgequake_webui/e2e/`
- UX/UI output dir: `ux_ui_map/` (repo root)

## Route discovery
Prefer enumerating concrete `page.tsx` files under `edgequake_webui/src/app/**/page.tsx`.
- Ignore route groups: folders like `(dashboard)` do not appear in URL paths.
- Avoid dynamic segments: paths with `[param]`, `[...param]`, `[[...param]]` are not capturable without fixture data.

Helper script (no dependencies):
- Derive concrete routes from the Next.js App Router: [scripts/derive_routes.mjs](scripts/derive_routes.mjs)

Example:

`node .github/skills/playwright-ux-ui-capture/scripts/derive_routes.mjs --format json`

## Viewports (required)
Capture exactly these 3 widths (use a reasonable height):
- desktop: 1440px
- tablet: 768px
- mobile: 375px

## Waiting strategy (important)
- Prefer Playwright auto-waits + a stable “page ready” signal (e.g. a visible `main` or `h1`).
- Avoid relying on `waitForLoadState('networkidle')` as the primary sync.

## Artifact write-out (must be immediate)
For each route:
1) create `ux_ui_map/screenshots/{page}/`
2) write `desktop.png`, `tablet.png`, `mobile.png`
3) write/update `ux_ui_map/requests/{page}.json` from the template
4) append one JSON line to `ux_ui_map/capture-index.jsonl`

## How to run
From repo root:
- `cd edgequake_webui && npm run test:e2e`

To run a single spec (preferred while iterating):
- `cd edgequake_webui && npx playwright test e2e/<spec>.spec.ts`

## Output example (capture index)
Each line is a standalone JSON object:

```json
{"page":"dashboard","route":"/","capturedAt":"2025-12-26T12:34:56.000Z","ok":true}
```

## Common failure handling
- If a page requires auth, capture `/login` first and then attempt the route.
- If a route 404s, still write a capture-index line with `ok:false` and store a screenshot of the error state.

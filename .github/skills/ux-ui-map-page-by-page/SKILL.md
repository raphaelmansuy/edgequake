---
name: ux-ui-map-page-by-page
description: Produce the EdgeQuake WebUI UX/UI map one route at a time (capture screenshots, then immediately write per-page docs and per-page analysis requests). Use when asked to map UI, capture screens page-by-page, avoid agent memory saturation, or generate ux_ui_map artifacts.
license: Proprietary (repository internal)
compatibility: Designed for GitHub Copilot agent mode in VS Code; requires Playwright E2E to run the Next.js dev server.
metadata:
  repo: raphaelmansuy/edgequake
  area: ux-ui-mapping
---

# UX/UI Map (Page-by-Page)

## When to use
Use this skill when the user asks to:
- capture screens **page by page**
- avoid “capture everything then analyze later”
- generate/update `ux_ui_map/` artifacts

## Key rule (memory-safe)
Process **exactly one route at a time**:
1. Navigate route
2. Capture screenshots (desktop/tablet/mobile)
3. Immediately write/update the page doc in `ux_ui_map/pages/`
4. Immediately write/update the page request in `ux_ui_map/requests/`
5. Only then move to the next route

Do not keep previous pages in working context.

## Output contract
Write these files as you go:
- `ux_ui_map/pages/{page}.md` (page documentation)
- `ux_ui_map/screenshots/{page}/...png` (screenshots)
- `ux_ui_map/requests/{page}.json` (analysis request inputs)
- `ux_ui_map/capture-index.jsonl` (append-only log, one JSON object per captured route)

Templates:
- Page doc template: [assets/page-template.md](assets/page-template.md)
- Request JSON template: [assets/request-template.json](assets/request-template.json)

Helper script:
- Scaffold/validate per-page artifacts: [scripts/page_artifacts.mjs](scripts/page_artifacts.mjs)

### Script usage

Scaffold the expected files/folders for one page:

`node .github/skills/ux-ui-map-page-by-page/scripts/page_artifacts.mjs scaffold --page dashboard --route /`

Validate that a page has its 3 required screenshots and docs:

`node .github/skills/ux-ui-map-page-by-page/scripts/page_artifacts.mjs validate --page dashboard`

## Naming
- `{page}` is a stable slug (e.g. `dashboard`, `documents`, `query`, `graph`, `settings`, `api-explorer`, `login`).
- Screenshot names include viewport suffix: `desktop.png`, `tablet.png`, `mobile.png`.

## Minimal checklist per route
- [ ] Route loads without hard failure (HTTP/blank page)
- [ ] 3 screenshots saved (desktop/tablet/mobile)
- [ ] `ux_ui_map/pages/{page}.md` created/updated
- [ ] `ux_ui_map/requests/{page}.json` created/updated
- [ ] Capture appended to `ux_ui_map/capture-index.jsonl`

## Example prompts that should trigger this skill
- “Capture the UI page-by-page and write analysis immediately.”
- “Update the UX/UI mapping, but don’t blow the context window.”
- “Generate `ux_ui_map/` artifacts for all routes.”

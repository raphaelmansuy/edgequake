---
name: ux-ui-analyze-single-page
description: Analyze exactly one captured UI page (from ux_ui_map screenshots + request JSON) and immediately write/update ux_ui_map/pages/{page}.md in neutral descriptive language. Use when asked to analyze screenshots, rewrite corresponding analysis immediately, or avoid memory/context saturation.
license: Proprietary (repository internal)
compatibility: Designed for Copilot agent mode; can be used without running the app if screenshots already exist.
metadata:
  repo: raphaelmansuy/edgequake
  area: ux-ui-mapping
---

# Single-Page UI Analysis (Write Immediately)

## When to use
Use this skill when:
- screenshots already exist and need describing
- the user wants analysis written out immediately after each page capture
- you must avoid carrying multi-page context

## Inputs
- `ux_ui_map/requests/{page}.json`
- `ux_ui_map/screenshots/{page}/desktop.png`
- `ux_ui_map/screenshots/{page}/tablet.png`
- `ux_ui_map/screenshots/{page}/mobile.png`

## Output
- `ux_ui_map/pages/{page}.md` (created/updated immediately)

## Rules
- Descriptive only: do not judge, do not recommend, do not assign severity.
- Focus on the hierarchy: page → regions → containers → components → elements.
- If information is uncertain from the screenshot, say so plainly (e.g., “Text is not legible at this resolution”).
- Do not reference previous pages unless explicitly asked.

## Procedure
1) Load the request JSON to confirm route and expected screenshot paths.
2) Inspect desktop screenshot first (structure), then tablet/mobile (responsive changes).
3) Write/update the page doc using the template at `ux_ui_map/pages/{page}.md`:
   - Overview (route/title)
   - ASCII layout diagram (approximate)
   - Screenshot table
   - Regions/containers/components
4) Stop after writing this one page.

## Example prompt that should trigger this skill
- “Analyze the dashboard screenshots and write the page doc now.”
- “Rewrite the corresponding analysis immediately for /documents.”

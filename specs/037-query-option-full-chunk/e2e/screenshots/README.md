# SPEC-037 E2E Screenshot Analysis

Captured by `edgequake_webui/e2e/spec037-query-*.spec.ts` into this directory.

| File | Test | Verification |
| ---- | ---- | ------------ |
| `01-settings-open-top.png` | Scroll — panel open | Context + Response Mode visible; System Prompt not yet in view |
| `02-settings-scrolled-system-prompt.png` | Scroll — bottom reached | **System Prompt** textarea fully visible after scroll (REQ-037-01) |
| `03-full-chunk-toggle-on.png` | Full chunk toggle | **Full passage text** switch ON in Response Mode section |
| `04-stream-response-agent-granularity.png` | API wire agent | Stream mock response; request carried `content_granularity: agent` |
| `05-citation-mode-stream.png` | API wire citation | Default/off toggle; request carried `content_granularity: citation` |

## Regenerate

```bash
cd edgequake_webui
PLAYWRIGHT_BASE_URL=http://localhost:3000 pnpm exec playwright test \
  e2e/spec037-query-settings-scroll.spec.ts \
  e2e/spec037-query-full-chunk.spec.ts
```

Screenshots are written via `spec037Screenshot()` in `e2e/helpers/screenshot-paths.ts`.

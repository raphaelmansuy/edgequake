# SPEC-017 WebUI — LaTeX markdown E2E index

| ID | Proof | Playwright spec |
|----|-------|-----------------|
| 001 | [LaTeX KaTeX rendering](./001-latex-rendering-proof.md) | `edgequake_webui/e2e/spec017-markdown-latex.spec.ts` |

## Run proof

```bash
# WebUI dev server (e.g. make dev on :3000 or PLAYWRIGHT webServer on :3001)
cd edgequake_webui
bunx playwright test e2e/spec017-markdown-latex.spec.ts
```

## Screenshots

- `screenshots/01-markdown-latex-fixture-full.png`
- `screenshots/02-markdown-latex-fixture-panel.png`

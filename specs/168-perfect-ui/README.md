# SPEC-168 — Perfect UI: audit, keyboard, CLS

> **Mission.** Make the EdgeQuake WebUI the most polished GraphRAG / knowledge
> management interface available: calm, fast, never clunky, fully keyboard
> operable, and visually stable (no layout shift).
>
> **Method.** Every screen is captured with Playwright against a deterministic,
> realistic dataset, **visually inspected**, fixed, and recaptured until the
> screenshots stop revealing defects. Keyboard and CLS are enforced by tests,
> not by opinion.

## Pack

| File | Purpose |
| ---- | ------- |
| [01-audit-findings.md](01-audit-findings.md) | Findings per screen/component with evidence (screenshot refs) |
| [02-design-principles.md](02-design-principles.md) | The bar every surface is held to |
| [03-keyboard-spec.md](03-keyboard-spec.md) | Global + per-surface keyboard contract |
| [04-cls-spec.md](04-cls-spec.md) | Layout-stability contract, budget, how it is measured |
| [05-iteration-ledger.md](05-iteration-ledger.md) | Round-by-round log: what was seen, what changed |
| `e2e/screenshots/round-NN-*/` | Screenshot evidence per round (light/dark × 375/768/1440) |
| `e2e/reports/` | Machine-readable CLS / keyboard / a11y reports per round |

## How to run

```bash
# 1. dev server on :3010 (the Playwright config reuses it)
cd edgequake_webui && bun run dev -- --port 3010

# 2. capture every route (round name = output folder)
SPEC168_ROUND=round-01 SPEC168_WIDTHS=375,768,1440 SPEC168_THEMES=light,dark \
  pnpm exec playwright test e2e/spec168/baseline.spec.ts --project=mock-api

# 3. gates
pnpm exec playwright test e2e/spec168/cls.spec.ts      --project=mock-api
pnpm exec playwright test e2e/spec168/keyboard.spec.ts --project=mock-api
```

All specs are hermetic (`e2e/spec168/helpers/rich-mock.ts`): no backend, no LLM,
deterministic data (`rich-data.ts`), so screenshots are comparable across rounds.

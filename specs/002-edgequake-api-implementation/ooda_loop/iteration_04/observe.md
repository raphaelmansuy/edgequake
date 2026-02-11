# Iteration 04: Observe — Build Verification & README Polish

## Date: 2026-02-11

## Observations

### Build Output Analysis

Ran `npm run build` (tsup) and verified dual-format output:

```
  ESM   dist/index.js       43.80 KB
  CJS   dist/index.cjs      44.33 KB
  DTS   dist/index.d.ts     62.82 KB
  DTS   dist/index.d.cts    62.82 KB
```

- ESM and CJS bundles are nearly identical size (~44KB)
- Type definitions are larger (~63KB) — expected given 131+ endpoint type surface
- Source maps generated for debugging
- No build warnings or errors

### TypeScript Strict Mode

`tsc --noEmit` passed with zero errors — all types are sound under strict mode.

### README State Before Polish

- 145 lines, good structure with features/install/quickstart/config/resources
- Missing: examples directory link, docs links, development commands
- License section said "MIT" — should be Apache-2.0 per project standard
- Error handling example used `RateLimitError` — actual export is `RateLimitedError`

### Package.json State

- `prepublishOnly` already set to `lint && test && build` (from iteration_03)
- License already `Apache-2.0` (from iteration_03)
- Engines constraint: `node >=18.0.0`
- `sideEffects: false` for tree-shaking

### Test State

- 243 tests, all passing
- 98.52% line coverage, 97.02% function coverage, 85.43% branch coverage

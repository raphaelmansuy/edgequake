# QueryMode backend parity (UI-DRY-005)

**Status:** ✅ Verified  
**Date:** 2026-06-04 18:40 UTC

## Evidence

- `types/query.ts`: `mix`, `bypass` in `QUERY_MODES`; `QUERY_MODES_SELECTOR` for UI chips.
- `query-mode-selector.tsx` driven by `QUERY_MODES_SELECTOR` (4 modes).
- Vitest: `query-mode.test.ts`.

## Verdict

Backend parity in types; UI intentionally exposes local/global/hybrid/naive only.

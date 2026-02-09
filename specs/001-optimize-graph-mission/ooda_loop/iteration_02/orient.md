# Iteration 02 - Orient

## Analysis

The 500 node limit is being bypassed through **localStorage persistence**. User's previous session allowed them to set a higher value (via auto-optimize or manual slider), which persisted.

### System Architecture Analysis

```
┌───────────────────┐     ┌──────────────────┐     ┌───────────────────┐
│   localStorage    │────▶│ Settings Panel   │────▶│   Graph Store     │
│ graph-max-nodes   │     │ Validates 10000  │     │ maxNodes state    │
│ Value: ~1700      │     │ Should be 500    │     │ Used by API calls │
└───────────────────┘     └──────────────────┘     └───────────────────┘
         │                                                    │
         │                                                    ▼
         │                                  ┌──────────────────────────┐
         │                                  │    graph.rs (backend)    │
         │                                  │ Returns requested nodes  │
         │                                  │ No hard cap enforcement  │
         │                                  └──────────────────────────┘
         │
         ▼
┌────────────────────────────────────────────────────────────────────┐
│                       Bug Flow Analysis                            │
├────────────────────────────────────────────────────────────────────┤
│ 1. User opens page with auto-optimize enabled (default)            │
│ 2. Auto-optimize detects high-tier device → maxNodes = 1000        │
│ 3. Value saved to localStorage                                     │
│ 4. Page reload → localStorage read → validation passes (≤10000)   │
│ 5. Graph fetches 1000+ nodes → UI overwhelmed                      │
└────────────────────────────────────────────────────────────────────┘
```

### DRY Violation Identified

The `MAX_DISPLAY_NODES = 500` constant exists in `use-graph-store.ts:37` but is NOT imported/used in:

- `graph-settings-panel.tsx` (hardcodes 10000)
- `auto-optimize.ts` (hardcodes 1000 for high tier)

This violates DRY - the source of truth should be the constant.

### Root Causes (Prioritized)

| Priority | File                          | Issue                                | Impact                       |
| -------- | ----------------------------- | ------------------------------------ | ---------------------------- |
| P0       | `graph-settings-panel.tsx:93` | localStorage validation allows 10000 | Persisted value bypasses cap |
| P1       | `auto-optimize.ts:75`         | High tier sets maxNodes to 1000      | Initial value exceeds cap    |
| P2       | `graph-settings-panel.tsx`    | Slider max not capped                | UI allows manual override    |

### Hypotheses

**H1**: Fixing localStorage validation to use MAX_DISPLAY_NODES will cap restored values at 500

- Test: Change `<= 10000` to `<= MAX_DISPLAY_NODES`, reload page, verify maxNodes ≤ 500

**H2**: Fixing auto-optimize will prevent new sessions from exceeding 500

- Test: Clear localStorage, reload, verify auto-optimize returns ≤ 500

**H3**: Backend should also enforce a hard cap as defense-in-depth

- Test: Set maxNodes to 9999 in requests, verify backend caps at 500

## Recommended Solution

Apply fixes in dependency order:

1. **First**: Fix `graph-settings-panel.tsx` localStorage validation (line 93)
2. **Second**: Fix `auto-optimize.ts` tier configurations (lines 73-77)
3. **Third**: Add backend defense-in-depth validation in `graph.rs`

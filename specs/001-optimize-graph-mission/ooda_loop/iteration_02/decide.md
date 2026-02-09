# Iteration 02 - Decide

## Decision Matrix

| Fix                         | Effort | Risk | Impact | Priority |
| --------------------------- | ------ | ---- | ------ | -------- |
| localStorage validation cap | Low    | Low  | High   | P0       |
| Auto-optimize cap           | Low    | Low  | High   | P1       |
| Backend defense-in-depth    | Medium | Low  | Medium | P3       |

## Chosen Actions

### Action 1: Fix localStorage validation (P0)

**File**: `graph-settings-panel.tsx` line 93
**Change**: Replace hardcoded `10000` with `MAX_DISPLAY_NODES`

```typescript
// Before:
if (!isNaN(parsed) && parsed >= 100 && parsed <= 10000) {

// After:
if (!isNaN(parsed) && parsed >= 100 && parsed <= MAX_DISPLAY_NODES) {
```

**Also requires**: Import MAX_DISPLAY_NODES from use-graph-store

### Action 2: Fix auto-optimize tier configurations (P1)

**File**: `auto-optimize.ts` lines 73-77
**Change**: Cap all tier maxNodes to MAX_DISPLAY_NODES

```typescript
// Before:
high: { maxNodes: 1000, ... }

// After:
high: { maxNodes: MAX_DISPLAY_NODES, ... }
```

### Action 3: Fix slider max value (P2)

**File**: `graph-settings-panel.tsx`
**Change**: Update slider max from 10000 to MAX_DISPLAY_NODES

### Deferred Action: Backend hard cap

**File**: `graph.rs`
**Why deferred**: Frontend fixes should be sufficient; backend change requires more testing

## Implementation Order

1. Add import for MAX_DISPLAY_NODES in `graph-settings-panel.tsx`
2. Fix localStorage validation (line 93)
3. Fix auto-optimize.ts tier maxNodes values
4. Update slider max value
5. Clear localStorage and test

## Success Criteria

- [ ] Page loads with ≤ 500 nodes displayed
- [ ] Auto-optimize never returns maxNodes > 500
- [ ] localStorage validation caps at 500
- [ ] Slider max is 500
- [ ] E2E test verifies node count ≤ 500

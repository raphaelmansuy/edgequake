# OODA-05: Pipeline Status Button Order - Orient Phase

## Analysis

### User Intent
The user's primary action when viewing pipeline status is to **dismiss the dialog and continue working**. Canceling the pipeline is a **rare, destructive action** that should require deliberate action.

### Current UX Problems

| Aspect | Current | Expected |
|--------|---------|----------|
| Close button variant | `outline` (secondary) | `default` (primary) |
| Cancel button variant | `destructive` (high weight) | `outline` (secondary) |
| Close position | Left | Right (default position) |
| Cancel position | Right | Left (secondary position) |
| Initial focus | None | Close button |

### Impact of Current Design
- Users might accidentally trigger Cancel Pipeline instead of Close
- Visual hierarchy draws attention to destructive action
- Violates standard dialog UX patterns (OK/Cancel dialogs have OK on right)

### Risk Assessment
- **Low risk**: Button order change is purely visual
- **No backend changes**: Frontend-only modification
- **Testable**: Can verify in browser immediately

## Decision: Reorder and Restyle Buttons

### New Layout
1. **Cancel Pipeline** → Left side, `variant="outline"` (secondary)
2. **Close** → Right side, `variant="default"` (primary), `autoFocus`

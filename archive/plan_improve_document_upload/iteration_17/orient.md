# Iteration 17: Enhanced Processing Animation - Orient

## Analysis

### Current State

The StatusBadge component uses `animate-spin` on the icon for processing states, but the badge itself remains static. This can make it less obvious that a document is actively being processed.

### Enhancement Approach

Adding `animate-pulse` to the entire Badge component creates a subtle pulsing effect that:

1. Draws attention to processing items
2. Provides visual feedback that the system is working
3. Reduces user uncertainty about whether processing is happening

### Implementation Details

- Conditional `animate-pulse` class based on `config.animate` flag
- Added `data-testid="status-badge"` for E2E testing
- Added OODA-17 documentation comment explaining the WHY

### Visual Effect

- Icon: animate-spin (rotates continuously)
- Badge: animate-pulse (subtle opacity pulse)
- Combined: Clear visual indication of active processing

## Risk Assessment

- Low risk: Tailwind's `animate-pulse` is a well-tested animation
- No layout shifts or performance concerns
- Animation is subtle enough not to be distracting

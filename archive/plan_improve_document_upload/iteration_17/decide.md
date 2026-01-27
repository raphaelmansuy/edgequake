# Iteration 17: Enhanced Processing Animation - Decide

## Decision

### Selected Enhancement

Add `animate-pulse` class to the Badge wrapper for processing states, creating a dual-animation effect:

- Icon spins (animate-spin)
- Badge pulses (animate-pulse)

### Implementation Changes

1. Modified status-badge.tsx line ~120
2. Added conditional `animate-pulse` to Badge className
3. Added `data-testid="status-badge"` for testability
4. Added OODA-17 documentation comment

### Code Change

```tsx
const badge = (
  <Badge
    variant="outline"
    className={`gap-1 ${config.textColor} border-current cursor-default ${
      config.animate ? "animate-pulse" : ""
    }`}
    data-testid="status-badge"
  >
    <Icon className={`h-3 w-3 ${config.animate ? "animate-spin" : ""}`} />
    {!compact && config.label}
  </Badge>
);
```

### Affected States

Pulse animation applies to:

- processing
- chunking
- extracting
- embedding
- indexing

### Non-affected States

No animation on:

- pending
- completed
- indexed
- failed
- cancelled

## Rationale

The dual animation (spin + pulse) provides clear visual differentiation between processing and non-processing states, reducing user uncertainty.

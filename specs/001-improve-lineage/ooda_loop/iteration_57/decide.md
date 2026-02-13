# Decision - Iteration 57

## Decision: NO GLOBAL CHANGE

Keep per-instance `!block` overrides rather than modifying ScrollArea globally.

### Rationale

1. Only one instance had a user-visible bug (graph right panel)
2. Global change risks breaking horizontal scroll use cases
3. Per-instance approach is explicit and documented
4. Any future overflow issues can reference iteration 51 for the fix pattern

# Iteration 04 - DECIDE Phase

## Decision: Implement Test IDs and Impact Preview

### Actions to Take

1. **Add data-testid attributes** to both rebuild buttons
   - Target: `rebuild-embeddings-button.tsx` and `rebuild-knowledge-graph-button.tsx`
   - Add test IDs to buttons, confirm actions, and cancel buttons

2. **Add document count hook** to fetch stats
   - Create `useDocumentStats` hook or use existing query
   - Fetch document count for selected workspace

3. **Enhance confirmation dialog** with impact preview
   - Show "This workspace has X documents (Y chunks)"
   - Add time estimate: "Estimated time: ~Z minutes"

4. **Update E2E tests** to use new test IDs

### Implementation Order

1. First: Add data-testid attributes (immediate value for testing)
2. Second: Add impact preview (UX improvement)

### Code Changes Required

#### rebuild-embeddings-button.tsx

```tsx
// Button
<Button data-testid="rebuild-embeddings-button" ...>

// Dialog actions
<AlertDialogCancel data-testid="rebuild-embeddings-cancel">
<AlertDialogAction data-testid="rebuild-embeddings-confirm">

// Add document count display in dialog description
```

#### rebuild-knowledge-graph-button.tsx

```tsx
// Button
<Button data-testid="rebuild-kg-button" ...>

// Dialog actions
<AlertDialogCancel data-testid="rebuild-kg-cancel">
<AlertDialogAction data-testid="rebuild-kg-confirm">
```

### Metrics

- New test IDs: 6 (3 per component)
- UX improvement: Show impact before confirmation
- Time estimate heuristic: 3 seconds per document

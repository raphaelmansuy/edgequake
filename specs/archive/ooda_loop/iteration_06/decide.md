# Iteration 06 - DECIDE Phase

## Decision: Create Error Handling E2E Test Suite

### Test File: `e2e/error-handling.spec.ts`

### Test Cases

1. **Error Popover Tests**
   - Error trigger is visible for failed documents
   - Clicking trigger opens popover
   - Popover shows full error message
   - Copy button shows success feedback
   - Retry button exists when handler provided
   - Popover closes on outside click

2. **Reprocess Failed Button Tests**
   - Button shows failed count
   - Button disabled when reprocessing
   - Confirmation dialog appears
   - Cancel closes dialog
   - Confirm starts reprocessing

3. **Document Status Tests**
   - Failed documents show red badge
   - Processing documents show spinner
   - Completed documents show green badge

### Implementation Plan

1. Create `e2e/error-handling.spec.ts`
2. Add data-testid to ReprocessFailedButton
3. Run tests to verify selectors work

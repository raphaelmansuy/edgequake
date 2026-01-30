# Decide - Iteration 09: Error Categorization Implementation

## Decision

Implement frontend error categorization with:

1. Pattern-based error categorization
2. Color-coded category display
3. Actionable suggestions
4. Retryable indicator for transient errors
5. Expandable technical details

## Implementation Plan

### 1. Create error-categories.ts

- Define ErrorCategory type
- Define CategorizedError interface
- Implement regex patterns for each category
- Export categorizeError() function
- Export getCategoryColor() helper
- Export getCategoryIcon() helper

### 2. Update ErrorMessagePopover

- Import categorization utilities
- Add useMemo for categorized error
- Display category icon and label
- Show "Retryable" badge for transient errors
- Add suggestion section with lightbulb icon
- Make technical details expandable

## Files to Change

- `src/lib/error-categories.ts` (NEW)
- `src/components/documents/error-message-popover.tsx` (UPDATE)

## Verification

- TypeScript compilation
- Visual inspection of error display
- E2E tests cover error popover

## Expected Outcome

Users see categorized errors with actionable suggestions
instead of raw technical error messages.

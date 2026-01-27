# Iteration 05 - DECIDE Phase

## Decision: Create ErrorMessagePopover Component

### Implementation Plan

1. **Create new component**: `src/components/documents/error-message-popover.tsx`

2. **Component Features**:
   - Clickable error text trigger
   - Popover with full error message
   - Copy to clipboard button
   - Toast feedback on copy
   - Styled for dark/light mode

3. **Integration**:
   - Replace inline error display in document-manager.tsx
   - Use in document-detail-dialog.tsx if applicable

### Component API

```tsx
interface ErrorMessagePopoverProps {
  message: string;
  documentId?: string;
  onRetry?: () => void;
  className?: string;
}
```

### Implementation Steps

1. Create `error-message-popover.tsx` with Popover + copy functionality
2. Update `document-manager.tsx` to use new component
3. Add data-testid for E2E testing
4. Verify TypeScript compilation

### Code Changes

**error-message-popover.tsx**:
- Import Popover components
- Use useState for copy feedback
- Implement clipboard API
- Style with tailwind

**document-manager.tsx**:
- Replace inline error JSX with ErrorMessagePopover
- Pass error message and retry callback

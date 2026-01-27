# Iteration 27 – DECIDE

## Decision

Enhance error toasts with retry actions and better descriptions.

## Implementation Plan

### 1. Pipeline Cancel Error

**File**: `pipeline-status-dialog.tsx`

Before:

```tsx
toast.error(`Failed to cancel: ${error.message}`);
```

After:

```tsx
toast.error(t("pipeline.cancelError", "Failed to cancel pipeline"), {
  description: error.message,
  action: {
    label: t("common.retry", "Retry"),
    onClick: () => cancelMutation.mutate(),
  },
});
```

### 2. Rebuild Errors

**Files**: `rebuild-embeddings-button.tsx`, `rebuild-knowledge-graph-button.tsx`

Add retry actions to all error handlers.

### 3. Pattern to Apply

```tsx
toast.error(t("key.error", "User friendly message"), {
  description: error instanceof Error ? error.message : undefined,
  action: {
    label: t("common.retry", "Retry"),
    onClick: () => mutation.mutate(),
  },
});
```

## Success Criteria

- [ ] All critical error toasts have retry actions
- [ ] Error messages include technical details in description
- [ ] User-friendly titles in all error toasts

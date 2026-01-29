# OODA-05: Pipeline Status Button Order - Decide Phase

## Decision

Modify the button layout in `pipeline-status-dialog.tsx` to make Close the default action.

## Changes Required

### File: `edgequake_webui/src/components/documents/pipeline-status-dialog.tsx`

**Lines 658-683** - Swap button order and update variants:

```tsx
{
  /* OODA-05: Reorder buttons - Close is default action (right side), Cancel is secondary (left side) */
}
<div className="flex gap-2">
  {/* Cancel Button - secondary action on the left */}
  <Button
    variant="outline"
    onClick={handleCancelClick}
    disabled={cancelMutation.isPending || data.cancellation_requested}
    className="flex-1"
  >
    {cancelMutation.isPending ? (
      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
    ) : (
      <XCircle className="mr-2 h-4 w-4" />
    )}
    {data.cancellation_requested
      ? t("pipeline.cancelPending", "Cancellation Pending...")
      : t("pipeline.cancel", "Cancel Pipeline")}
  </Button>
  {/* Close Button - default action on the right */}
  <Button
    variant="default"
    onClick={() => onOpenChange(false)}
    className="flex-1"
    autoFocus
  >
    {t("common.close", "Close")}
  </Button>
</div>;
```

## Key Changes

1. **Close button moved right** → Standard position for default action
2. **Close variant changed** → `outline` → `default`
3. **Cancel variant changed** → `destructive` → `outline`
4. **autoFocus added** → Close button receives initial keyboard focus

## No Backend Changes Required

This is a frontend-only UX improvement.

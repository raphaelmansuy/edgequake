# OODA-05: Pipeline Status Button Order - Observe Phase

## Observation Date: 2025-01-28

## Issue Identified

From mission requirements:

> "Pipeline status must be improved --> the default button must be close not cancel"

## Current State Analysis

### Button Layout (Before)

Location: `edgequake_webui/src/components/documents/pipeline-status-dialog.tsx`, lines 658-683

```tsx
<div className="flex gap-2">
  <Button
    variant="outline"
    onClick={() => onOpenChange(false)}
    className="flex-1"
  >
    {t('common.close', 'Close')}
  </Button>
  {/* Cancel Button - stops the rebuild */}
  <Button
    variant="destructive"
    onClick={handleCancelClick}
    ...
  >
    {t('pipeline.cancel', 'Cancel Pipeline')}
  </Button>
</div>
```

### Issues

1. **Close button is secondary** (outline variant)
2. **Cancel button is primary** (destructive variant = high visual weight)
3. **No autoFocus** on Close button
4. **Dialog UX convention violated** - default action should be right-most, primary styled

## UX Best Practices

1. **Default action** = most common/safe action = Close (dismiss without side effects)
2. **Destructive action** = Cancel Pipeline (stops processing) should be secondary
3. **Button order** = secondary left, primary right (matches dialog conventions)
4. **Visual hierarchy** = Default button should have `variant="default"`, secondary `variant="outline"`
5. **Keyboard focus** = Default button should receive initial focus (`autoFocus`)

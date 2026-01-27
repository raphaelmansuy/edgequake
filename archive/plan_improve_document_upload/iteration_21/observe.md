# Iteration 21: Document Preview Error Enhancement - Observe

## Current State Analysis

### Error Display in Preview Panel
Location: document-preview-panel.tsx lines 424-438

Current implementation:
```tsx
{isFailed && document.error_message && (
  <>
    <Separator />
    <div className="space-y-2">
      <h4 className="text-sm font-medium text-destructive flex items-center gap-1.5">
        <AlertCircle className="h-4 w-4" />
        {t('documents.preview.error', 'Processing Error')}
      </h4>
      <Card className="bg-destructive/5 border-destructive/20">
        <CardContent className="p-3">
          <p className="text-xs text-destructive">{document.error_message}</p>
        </CardContent>
      </Card>
    </div>
  </>
)}
```

Issues:
- Raw error message without categorization
- No suggestions for resolution
- No indication if error is retryable

### Enhancement Opportunity
Integrate error-categories.ts to:
- Show category icon and color
- Display helpful suggestions
- Show retryable indicator
- Provide retry button for retryable errors

### Files to Modify
- src/components/documents/document-preview-panel.tsx
  - Import error categorization functions
  - Enhance error display section

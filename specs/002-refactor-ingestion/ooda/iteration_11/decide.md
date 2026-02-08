# OODA-11: Decide

## Action Plan

1. **Create `processing-status-summary.tsx`** component
   - Props: pipelineStatus, documents, onOpenDetails
   - Self-contained visibility check
   - Translation support

2. **Update `document-manager.tsx`**
   - Import new component
   - Replace inline section with `<ProcessingStatusSummary>` usage

3. **Verify**
   - TypeScript check
   - Verify visibility condition works

## Expected Outcome
- **Lines saved**: ~40 lines from DocumentManager
- **New component**: ~70 lines (reusable)
- **Target**: DocumentManager 1444 → ~1405 lines

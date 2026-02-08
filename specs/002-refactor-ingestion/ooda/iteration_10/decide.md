# OODA-10: Decide

## Action Plan

1. **Create `quick-action-buttons.tsx`** component
   - Props interface with document and callback handlers
   - Status constant arrays
   - Tooltip-wrapped action buttons

2. **Update `document-manager.tsx`**
   - Import new component
   - Replace inline buttons with `<QuickActionButtons>` usage
   - Pass DocumentActionsMenu as children

3. **Verify**
   - Run TypeScript check
   - Verify buttons work in UI

## Expected Outcome

- **Lines saved**: ~70 lines
- **New component**: ~75 lines (reusable across views)
- **Net reduction**: DocumentManager 1519 → ~1450 lines

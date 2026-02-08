# OODA-12: Decide

## Action Plan

1. **Create `document-table-states.tsx`** component
   - Handles both loading skeleton and empty state
   - Returns null when neither condition applies
   - Props: isLoading, isEmpty, onUploadClick

2. **Update `document-manager.tsx`**
   - Import new component
   - Replace inline conditional rendering

3. **Verify**
   - TypeScript check
   - Both states render correctly

## Expected Outcome

- **Lines saved**: ~25 lines from DocumentManager
- **New component**: ~55 lines
- **Target**: DocumentManager 1399 → ~1375 lines

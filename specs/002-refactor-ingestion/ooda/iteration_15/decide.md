# OODA-15 Decide: Action Plan

## Decision

Extract table row to `DocumentTableRow` component with simplified prop interface.

## Action Plan

### Step 1: Create Component File

- Path: `components/documents/document-table-row.tsx`
- Include:
  - JSDoc documentation
  - Props interface
  - Helper functions (getFileTypeIcon, highlightMatches)
  - Memoized component

### Step 2: Update DocumentManager

1. Add import for DocumentTableRow
2. Replace inline row rendering with component usage
3. Remove helper functions if moved to component

### Step 3: Verify

- Run `get_errors` to check TypeScript
- Count lines to confirm reduction

### Step 4: Commit

- Message: `OODA-15: Extract DocumentTableRow component`

## Expected Outcome

- DocumentManager: 988 → ~893 lines (-95)
- New component: ~200 lines
- Cumulative reduction: 929 lines (51%)

## Next: Act

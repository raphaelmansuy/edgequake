# OODA-14 Decide: Action Plan

## Decision
Extract all document mutations to `use-document-mutations.ts` hook.

## Action Plan

### Step 1: Create Hook File
- Path: `edgequake_webui/src/hooks/use-document-mutations.ts`
- Include:
  - JSDoc documentation
  - Options interface
  - Return interface
  - All 4 mutations
  - Convenience `isAnyMutationPending` flag

### Step 2: Update DocumentManager
1. Add import for useDocumentMutations
2. Replace inline mutation definitions with hook call
3. Remove unused imports if any

### Step 3: Verify
- Run `get_errors` to check TypeScript
- Count lines to confirm reduction

### Step 4: Commit
- Message: `OODA-14: Extract useDocumentMutations hook`

## Expected Outcome
- DocumentManager: 1064 → ~974 lines (-90)
- New file: ~150 lines
- Cumulative reduction: 848 lines (46.5%)

## Next: Act

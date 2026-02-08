# OODA-16 Decide: Action Plan

## Decision

Extract bulk selection state and handlers to `use-bulk-selection.ts` hook.

## Action Plan

### Step 1: Create Hook File

- Path: `edgequake_webui/src/hooks/use-bulk-selection.ts`
- Include:
  - JSDoc documentation
  - Options and return interfaces
  - Internal selectedIds state
  - All selection handlers
  - Bulk operation handlers with loading states

### Step 2: Update DocumentManager

1. Add import for useBulkSelection
2. Replace inline state and handlers with hook call
3. Remove unused state declaration
4. Update props to BatchActionsBar

### Step 3: Verify

- Run `get_errors` to check TypeScript
- Count lines to confirm reduction

### Step 4: Commit

- Message: `OODA-16: Extract useBulkSelection hook`

## Expected Outcome

- DocumentManager: 841 → ~766 lines (-75)
- New hook: ~180 lines
- Cumulative reduction: 1056 lines (58%)

## Next: Act

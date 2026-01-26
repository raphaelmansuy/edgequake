# OODA-37: Decide

## Action Plan

1. Add `test_delete_does_not_affect_other_workspaces`
   - Create docs in workspace A and B
   - Delete doc in A
   - Verify doc in B still exists

2. Add `test_delete_same_name_different_workspaces`
   - Create same-named doc in A and B
   - Delete in A
   - Verify B's doc remains

## Success Criteria

- Both tests pass
- Total deletion tests: 50

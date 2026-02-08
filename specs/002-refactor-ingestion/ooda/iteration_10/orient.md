# OODA-10: Orient

## Analysis

### SRP Assessment
The quick action buttons are row-level UI controls that:
- Have their own rendering logic
- Require document status checks
- Handle multiple actions with tooltips
- Are independent of table structure

**Verdict**: Clear SRP candidate - row actions belong in their own component.

### Pattern Comparison
Similar to `DocumentActionsMenu` (OODA-09):
- Both operate on single document
- Both have conditional rendering based on status
- Both use Tooltip UI pattern

### Integration Points
1. **Parent**: Table row renders `<QuickActionButtons>`
2. **Sibling**: `DocumentActionsMenu` can be passed as children
3. **Handlers**: All click handlers come from parent props

### Risk Analysis
- **Low Risk**: Pure UI extraction, no state management
- **Type Safety**: Props interface ensures proper typing
- **Test Impact**: Can unit test button visibility independently

# OODA-12: Orient

## Analysis

### SRP Assessment
Loading and empty states:
- Are mutually exclusive with actual content
- Have their own rendering logic
- Can be reused in other table views
- Are presentation-only (no business logic)

**Verdict**: Good SRP candidate - table state displays are distinct from data rendering.

### Pattern Alignment
Follows react pattern of conditional rendering components:
- Clear prop interface
- No side effects
- Easily testable in isolation

### Alternative Considered
Could make two separate components (TableSkeleton, TableEmptyState) but:
- They're always used together in conditional logic
- Combined component simplifies usage at call site
- Both relate to "no data to display" states

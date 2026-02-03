# OODA-19: Decide Phase

## Decision

Implement **Option A: Filter Rotated Text** as a first step.

### Rationale

1. **Immediate Win**: Removes incorrect inline merging
2. **Low Risk**: Filtering is safe (no data corruption)
3. **Incremental**: Can enhance later to relocate text

### Expected Outcome

- **Positive**: No more inline arXiv identifiers in paragraphs
- **Limitation**: arXiv metadata missing from output (gold file expects it at top)
- **Quality Impact**: Potentially neutral (fixes inline issue, but misses expected content)

### Future Enhancement (Option B)

To fully match gold file:

1. Collect rotated elements separately
2. Identify arXiv watermarks by pattern: `arXiv:\d+\.\d+v\d+`
3. Insert as bold line after title
4. Remove ⋆ footnote markers from output

## Test Plan

1. Run quality test suite
2. Verify agent document no longer has inline arXiv
3. Check if quality improved or remained stable

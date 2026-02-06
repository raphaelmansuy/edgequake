# OODA-IT42 Orient: Disable Table Processors

## Analysis of Options

### Option A: Fix TableDetectionProcessor heuristics
- Requires understanding complex column alignment algorithms
- Would need test cases for all table variations (merged cells, spanning headers, etc.)
- High effort, uncertain outcome

### Option B: Add post-validation to reject bad tables
- Already added column count validation in `render_table_from_children()`
- Validation passes but content mapping is still wrong
- The problem is earlier in the pipeline, not at render time

### Option C: Disable table processors entirely
- Simple, reversible change
- Tables render as plain text paragraphs
- Preserves content, loses formatting
- Can be re-enabled once proper table reconstruction is implemented

## Decision
**Option C**: Disable both `TableDetectionProcessor` and `TextTableReconstructionProcessor`

## Rationale (First Principles)
1. **Primum non nocere** (first, do no harm): Garbled tables misinform, plain text informs
2. **Reversibility**: Processors can be re-enabled when fixed
3. **Validation**: Output file size increased 57266→61065 bytes, confirming content preserved

## Implementation
- Comment out processor additions in `build_default_chain()`
- Remove unused imports to satisfy clippy
- Document decision with OODA-IT42 comments

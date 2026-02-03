# OODA-15: Decide

## Decision: Implement Column-Aware Block Formation

### Approach

Modify the block formation logic in the extraction engine to:

1. Detect significant horizontal gaps within the same row
2. Create separate blocks for each column "cell"
3. Preserve column boundaries for downstream table detection

### Implementation Plan

1. **Identify block formation code** in `backend/extraction_engine.rs`
2. **Add column gap detection** during text element grouping:
   - Calculate gaps between adjacent elements in same Y-row
   - Use threshold: 30-40pt (typical column gap in 2-column tables)
3. **Create separate blocks** when gap exceeds threshold
4. **Preserve existing merging** for normal paragraph text

### Threshold Selection (First Principles)

**Column Gap Threshold: 30pt**

Reasoning:

- Typical word spacing: 3-8pt
- Typical sentence spacing: 8-12pt
- Typical column gap: 20-50pt (varies by document)
- 30pt is 3-4x word spacing = clearly intentional column break

### Expected Impact

- Table cells extracted as separate blocks
- TableDetectionProcessor detects multi-block rows
- Tables rendered with proper markdown structure
- AlphaEvolve Structure: 76.2% → ~85%+ expected

### Risk Mitigation

1. Only split when gap is significantly larger than word spacing
2. Apply heuristic: at least 2 rows with similar column structure
3. Fall back to paragraph if uncertain

### Files to Modify

1. `backend/extraction_engine.rs` - Block formation logic
2. Possibly `processors/table_detection.rs` - Adjust thresholds

### Testing Strategy

1. Run AlphaEvolve extraction, verify Table 1 detected
2. Run full quality suite, verify no regression
3. Check Goyal document tables still work

# OODA Loop 4: Decide

## Decisions

### D1: Pre-compute Document Frequencies
Add `compute_document_frequencies()` that builds DF map in single pass.

### D2: Factor Out IDF Formula
Create `compute_idf_from_df(n, df)` for O(1) IDF calculation from DF.

### D3: Keep Original compute_idf
Mark as `#[allow(dead_code)]` for backward compatibility and testing.

### D4: Use HashSet for Unique Terms
Ensure repeated terms in a document count only once for DF.

### D5: Add Verification Tests
- Test DF computation accuracy
- Test IDF equivalence between old and new methods
- Test repeated term handling
- Test edge cases (all docs, no docs)

## Risk Mitigation
- Equivalence test verifies identical results
- All existing tests continue to pass
- No API changes required

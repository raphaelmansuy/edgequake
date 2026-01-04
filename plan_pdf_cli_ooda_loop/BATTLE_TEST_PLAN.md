# Battle Testing Plan - 20 OODA Loops

**Objective**: Stress-test PDF converter with diverse real-world scenarios  
**Date**: 2026-01-04  
**Status**: In Progress

## Test Categories

### Phase 1: Real Academic Papers (Loops 1-5)
- **Loop 1**: Test on 2900_Goyal_et_al.pdf (VALIDATED - tables working ✅)
- **Loop 2**: Test on AlphaEvolve.pdf (44 pages, complex)
- **Loop 3**: Test on additional academic papers from real_dataset
- **Loop 4**: Test papers with heavy math notation
- **Loop 5**: Test papers with complex figures and captions

### Phase 2: Synthetic Test Suite (Loops 6-10)
- **Loop 6**: Test basic text and paragraphs (001-010)
- **Loop 7**: Test headings and structure (011-020)
- **Loop 8**: Test lists and formatting (021-030)
- **Loop 9**: Test tables and code blocks (031-039)
- **Loop 10**: Regression test all synthetic PDFs

### Phase 3: Edge Cases (Loops 11-15)
- **Loop 11**: Unicode and special characters
- **Loop 12**: Rotated text and transformations
- **Loop 13**: Encrypted/protected PDFs
- **Loop 14**: Scanned PDFs (OCR scenarios)
- **Loop 15**: Malformed/corrupted PDFs

### Phase 4: Complex Layouts (Loops 16-20)
- **Loop 16**: Multi-column academic papers
- **Loop 17**: Mixed orientations (portrait/landscape)
- **Loop 18**: Nested tables and complex structures
- **Loop 19**: Large documents (100+ pages)
- **Loop 20**: Full regression suite + performance metrics

## Success Criteria
- ✅ Table extraction working (ACHIEVED in debug session)
- 🎯 90%+ accuracy on synthetic tests
- 🎯 80%+ accuracy on real-world papers
- 🎯 No crashes or panics
- 🎯 Clear documentation of limitations

## Metrics to Track
1. Extraction success rate
2. Table detection accuracy
3. Heading hierarchy preservation
4. List indentation correctness
5. Unicode handling
6. Processing time per page
7. Memory usage

## Commit Strategy
- Commit after each phase (every 5 loops)
- Tag major breakthroughs
- Document regressions immediately
- Keep OODA structure updated

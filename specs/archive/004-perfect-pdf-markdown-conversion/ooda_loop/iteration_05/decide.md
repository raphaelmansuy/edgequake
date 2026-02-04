# OODA-05 Decision: Fix Title Extraction

## Issue Summary

The title "THE HOT MESS OF AI: HOW DOES MISALIGNMENT SCALE WITH MODEL INTELLIGENCE AND TASK COMPLEXITY?" was completely missing from the extraction output.

## Root Cause Analysis

Two issues were identified:

### Issue 1: TJ Kerning Threshold Too Low

The PDF uses TJ arrays with kerning values to position text:

- **Letter kerning**: -61 to -63 (typical adjustments between letters)
- **Word spaces**: -300 to -534 (actual word breaks)

The original threshold of `-50` was too low, causing:

- Every kerning adjustment to be interpreted as a word space
- Text like "THE" to become "T H E"

### Issue 2: Spaced Text Filtered as Garbled

When "T H E H O T M E S S..." was processed:

1. `GarbledTextFilterProcessor` ran early
2. It detected many isolated single letters (T, H, E, M, etc.)
3. The filter removed the title blocks as "garbled text"
4. `PostProcessor.fix_spaced_text()` ran AFTER, but blocks were already gone

## Decision

1. **Change TJ kerning threshold** from -50 to -150
   - This is the midpoint between -63 (letter kerning) and -300 (word space)
   - Words will be properly spaced without false letter separation

2. **Add SpacedTextProcessor** that runs BEFORE GarbledTextFilter
   - Fix spaced text patterns early in the pipeline
   - Prevent garbled filter from misidentifying title text

## Processor Chain Order

```
0. SpacedTextProcessor    # OODA-05: Fix spaced text FIRST
1. MarginFilter           # Remove page numbers, headers
2. GarbledTextFilter      # Remove noise (after spaced text fixed)
3. LayoutProcessor        # Establish block structure
...
11. PostProcessor         # Final cleanup
```

## Expected Result

- Title appears correctly: "THE HOT MESS OF AI: HOW DOES MISALIGNMENT..."
- Word spaces preserved where TJ kerning > 150
- No false spaces from letter kerning

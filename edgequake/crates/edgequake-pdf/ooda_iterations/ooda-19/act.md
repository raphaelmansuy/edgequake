# OODA-19: Act Phase

## Implementation Summary

### Changes Made

1. **Added `is_rotated` field to `TextElement`** (`backend/elements.rs`):

   ```rust
   pub struct TextElement {
       // ... existing fields
       /// OODA-19: Flag for rotated text (e.g., arXiv watermarks in margins)
       pub is_rotated: bool,
   }
   ```

2. **Added CTM rotation detection** (`backend/content_parser.rs`):

   ```rust
   fn is_rotated_ctm(ctm: &[f32; 6]) -> bool {
       let a = ctm[0].abs();
       let d = ctm[3].abs();
       // If both a and d are small (< 0.1), text is rotated ~90°
       a < 0.1 && d < 0.1
   }
   ```

3. **Filter rotated elements** (`backend/extraction_engine.rs`):

   ```rust
   let rotated_elements: Vec<_> = elements.iter().filter(|e| e.is_rotated).cloned().collect();
   let elements: Vec<_> = elements.into_iter().filter(|e| !e.is_rotated).collect();
   ```

4. **Updated test helpers** (5 files):
   - Added `is_rotated: false` to all `make_element()` functions

## Results

### Before Fix

```
arXiv:2510.09244v1 [cs.AI] 10 Oct 2025 Today, one can develop remarkable systems...
```

arXiv identifier incorrectly merged inline with paragraph.

### After Fix

```
Today, one can develop remarkable systems without the need to write complex...
```

arXiv identifier removed from body text.

### Logging Output

```
OODA19-ROTATED: Page 1 has 1 rotated text elements (filtered out)
  ROTATED: Y=440.7 X=32.0 text='arXiv:2510.09244v1  [cs.AI]  10 Oct 2025'
```

## Quality Impact

| Metric        | Before | After | Change |
| ------------- | ------ | ----- | ------ |
| Agent Overall | 80.1%  | 80.1% | 0%     |
| Aggregate     | 86.5%  | 86.5% | 0%     |

**Analysis**: Quality unchanged because:

1. Fixed: No more inline arXiv text in paragraphs
2. New Issue: arXiv text missing entirely (gold expects it at top)
3. Net Effect: One error replaced with another

## Next Steps

1. Enhance to relocate arXiv watermark to document start (Option B)
2. Investigate ⋆ footnote marker handling
3. Continue to OODA-20 with other low-scoring documents

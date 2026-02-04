# OODA Iteration 36 - Observe Phase

## Date: 2026-02-04

## Objective

Investigate missing Figure 4 and Figure 7 captions in 01_2512 extraction.

## Observations

### Missing Figure Captions

| Figure   | Present in Gold | Present in Ours | Content                          |
| -------- | --------------- | --------------- | -------------------------------- |
| Figure 4 | ✅ Line 549     | ❌ Missing      | "Cam×Time dataset visualization" |
| Figure 7 | ✅ Line 902     | ❌ Missing      | "Temporal compression ablation"  |

### Figure 4 Content in Gold

```
Figure 4. Cam×Time dataset visualization. (Top) A space-time
grid defined by a camera trajectory c = [c1, ..., cF] and animation
status t = [t1, ..., tF]. Cam×Time renders images for all (c, t)
pairs, covering the full grid for learning disentangled spatial and
temporal control. Any two sampled sequences of F frames from
the grid can form a source-target pair. (Bottom) One typical choice
of source videos is taking the diagonal cells in green.
```

### Search Results in Our Extraction

- "dataset visualization" → NOT FOUND
- "grid defined by" → NOT FOUND
- "camera trajectory c =" → NOT FOUND

The Figure 4 caption content is **completely missing** from our extraction.

### Hypothesis

Figure 4 is likely on a page where:

1. The figure itself takes up most of the page
2. The caption is in a smaller font or unusual position
3. Our Y-position thresholds might be classifying it as footer/header

### Page Layout Analysis Needed

The 01_2512 PDF is 17 pages. Figure 4 is on page 5 (based on gold line count).
Need to investigate:

1. What page is Figure 4 on in the PDF?
2. What is the Y-position of the caption?
3. Why was it classified as footer/header/excluded?

### Quality Impact

Missing 2 figure captions (~100-200 words) contributes to the recall gap:

- 9759 words (gold) - 7422 words (ours) = 2337 missing words
- 2 figure captions ≈ 100 words → ~4% of the gap
- Remaining 96% is from other sources (math notation, inline annotations)

## Next Steps

1. Extract the 01_2512 PDF with debug logging enabled
2. Find which page Figure 4 appears on
3. Check if caption is being classified as footer/header
4. Fix the classification threshold if needed

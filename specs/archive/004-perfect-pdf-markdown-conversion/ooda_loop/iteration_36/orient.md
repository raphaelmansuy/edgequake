# OODA Iteration 36 - Orient Phase

## Date: 2026-02-04

## Analysis

### Issue Diagnosis

| Figure   | Issue                         | Root Cause                        |
| -------- | ----------------------------- | --------------------------------- |
| Figure 4 | Caption broken into table     | Table detection capturing caption |
| Figure 7 | Caption merged with Figure 15 | Text ordering/merging issue       |

### Detailed Analysis

#### Figure 4

Our extraction has:

```
| vides | full-grid | rendering | (Figure | 4), | enabling | target | videos | to | sam- |
```

This is the Table 1 content that REFERENCES Figure 4, not the Figure 4 caption itself.

The actual Figure 4 caption text is:

```
Figure 4. Cam×Time dataset visualization. (Top) A space-time
grid defined by a camera trajectory c = [c1, ..., cF] and animation
status t = [t1, ..., tF].
```

This text is **completely missing** from our extraction.

**Hypothesis**: Figure 4 caption is on a page dominated by a large figure image.
The caption may be:

1. At an unusual Y position (very bottom)
2. In a smaller font than body text
3. Being classified as footer content

#### Figure 7

Our extraction has Figure 7's content merged with Figure 15:

```
...whereas training additionally with dataset. (Bottom) We compare
several time-embedding strategies. The MLP fails to lock...
```

This is Figure 7's caption content ("MLP fails to lock") appearing in the middle of
Figure 15's content ("training additionally with dataset").

**Root Cause**: Text from different pages or figures being interleaved due to:

1. Y-position similarity across pages
2. Incorrect page separation
3. Multi-page figure caption handling

### Impact Assessment

Missing content contributes to recall gap (0.770):

- Figure 4 caption: ~60 words
- Figure 7 caption: ~30 words
- Total: ~90 words (~1% of gap)

The main gap (~23%) must come from other sources.

### ASCII Diagram: Text Flow Issue

```
Gold File (Correct):                Our Extraction (Broken):

Page 5:                             Page 5:
┌─────────────────────┐            ┌─────────────────────┐
│ [Figure 4 Image]    │            │ [Figure 4 Image]    │
│                     │            │                     │
│ Figure 4. Cam×Time  │            │ (caption missing!)  │
│ dataset visual...   │ ──────────▶│                     │
└─────────────────────┘            └─────────────────────┘
                                    ⚠️ Caption filtered as footer?

Page 8:                             Page 8:
┌─────────────────────┐            ┌─────────────────────┐
│ [Figure 7 Image]    │            │ [Figure 7 Image]    │
│                     │            │                     │
│ Figure 7. Temporal  │            │ (caption merged     │
│ compression...      │ ──────────▶│  with Fig 15!)      │
└─────────────────────┘            └─────────────────────┘
```

### Next Steps

1. Investigate footer threshold calculation
2. Check if figure caption font size is triggering header/footer detection
3. Consider special handling for "Figure N." pattern

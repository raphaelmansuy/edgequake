# OODA-11 ORIENT: Root Cause Analysis of Structural Fidelity Gap

## Root Cause Identified

The `ListDetectionProcessor` in `structure_detection.rs` (lines 320-323) has two regex patterns:

```rust
let bullet_regex = Regex::new(r"^[-–—*•◦▪]\s+").unwrap();
let number_regex = Regex::new(r"^\d+[\.)]\s+").unwrap();
```

These patterns detect:

- ✅ Bullet lists: `- item`, `* item`, `• item`
- ✅ Numbered lists: `1. item`, `2) item`

But they **DO NOT** detect:

- ❌ Academic references: `[1] Author, "Title"...`
- ❌ Bracketed numbered lists: `[2] Some item`

## Impact on v2 PDF

The v2 PDF (v2_2512.25072v1.pdf) is an academic arXiv paper with:

- **44 references** in `[N]` format
- Current structural fidelity: **47.2%** (worst of all PDFs)
- Expected improvement: +30% structural fidelity with proper list detection

## Evidence

Gold file (v2_2512.25072v1.gold.md) contains:

```markdown
- [1] L. Sentis and O. Khatib, "A whole-body control framework..."
- [2] X. Cheng, J. Li, S. Yang, "Open-television..."
  ...
- [44] V. Makoviychuk et al., "Isaac gym..."
```

Current extraction:

```markdown
## REFERENCES

humanoids operating in human environments," in ICRA., 2006. Teleoperation...
[26] Z. Fu, Q. Zhao, Q. Wu, G. Wetzstein...
[3] X. Gu, Y.-J. Wang, and J. Chen, "Humanoid-gym...
```

Problems:

1. No list markers (`* `) prefix
2. Two-column cross-merge (left column reference [26] mixed with right column [3])
3. Reference entries running together without line breaks

## Structural Element Breakdown

| Element    | Gold | Extracted | Gap      | Fix Priority                   |
| ---------- | ---- | --------- | -------- | ------------------------------ |
| List Items | 85   | 22        | 63 (74%) | **HIGH - Reference detection** |
| Tables     | 45   | 17        | 28 (62%) | MEDIUM - Multi-column tables   |
| Headers    | 27   | 19        | 8 (30%)  | LOW - Mostly correct           |

## Proposed Fix

Modify `ListDetectionProcessor` in `structure_detection.rs` to add:

```rust
// Academic reference pattern: [N] where N is 1-3 digits
let ref_regex = Regex::new(r"^\[\d{1,3}\]\s+").unwrap();
```

This will detect:

- `[1] Author...` → ListItem
- `[2] Author...` → ListItem
- `[44] Author...` → ListItem

## Expected Outcome

With proper reference list detection:

- v2 PDF: 47.2% → ~75% structural fidelity (+27.8%)
- 01_2512.25075v1: 50.4% → ~70% structural fidelity (if similar)
- Overall average: 69.0% → ~80%+

## Additional Consideration

The two-column cross-merge issue in the References section is a separate problem:

- References section uses two-column layout on last pages
- Our column detection merges left `[1], [3], [5]...` with right `[2], [4], [6]...`
- This requires investigation of column handling for reference sections

## Next: DECIDE Phase

1. Add `[N]` pattern to ListDetectionProcessor
2. Test on v2 PDF to verify improvement
3. If needed, investigate column handling for reference sections

# DECIDE.md - Iteration 003

## Patch scope (targeted, high ROI)

1. **Backend**: Build multi-span lines (style runs) instead of a single span per block, so the Markdown renderer can preserve bold/italic within a line.
2. **Headings**: Make `HeaderDetectionProcessor` re-evaluate existing `SectionHeader` blocks and add numeric heading regex (`^\d+(\.\d+)*\s+...`) for section headings.
3. **Tables**: Tighten caption-adjacent scan bounds and improve header selection + add a small, deterministic parser for the common collapsed leaderboard line:
   `Agent Pipeline Func-IoU(%) Resolved(%) ...`

## Acceptance checklist

- [ ] `cd edgequake && cargo test -p edgequake-pdf` passes
- [ ] Validator report shows improved StyleAccuracy and TableAccuracy with no robustness regression
- [ ] `one_tool_2512.20957v2.mdf.gen`:
  - title becomes `# ...`
  - `1. Introduction` becomes `## 1. Introduction`
  - Table 3 no longer turns into a “Value” table of unrelated paragraphs


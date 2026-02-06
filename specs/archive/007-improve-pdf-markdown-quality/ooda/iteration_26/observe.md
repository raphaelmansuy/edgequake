# OODA Iteration 26 – Observe

## Issue: Page break horizontal rules (`---`) in output

Current output contains `---` between pages (3 occurrences in the AI_Services document). The gold standard does not include page break markers.

## Impact

- 168 → 160 lines after removing page breaks (8 lines: 3 `---` + surrounding blank lines)
- Page breaks fragment semantic content for LLM consumption
- Gold standard treats the PDF as continuous text flow

## Source

`render()` in markdown.rs line ~1655: `output.push_str("\n---\n\n")` when `self.style.page_breaks == true` and `i > 0` (not first page).

Default style has `page_breaks: true`.

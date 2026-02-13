# Analysis - Iteration 24

## Gaps Identified
1. No WebUI component for lineage export — backend endpoint exists but no UI to trigger it
2. API client missing `exportDocumentLineage()` function
3. Sidebar has no "Export" section for download buttons

## Recommendation
Add `LineageExport` component with JSON and CSV download buttons, integrate into metadata sidebar as a new collapsible section, and add `exportDocumentLineage()` to the API client.

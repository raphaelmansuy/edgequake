# Analysis - Iteration 22

## Gaps Identified
1. Mission deliverable #4 requires "Export Capability: Download complete lineage as JSON/CSV"
2. No endpoint returns lineage as a downloadable file with Content-Disposition headers
3. No CSV serialization for hierarchical lineage data
4. No format query parameter on any lineage endpoint

## Possible Solutions

### Solution A: New `/lineage/export` endpoint with format query param
- Pros: Clean separation, existing endpoints unchanged, supports both JSON and CSV
- Cons: One more route to maintain
- Risk: Low — additive change, no breaking modifications

### Solution B: Add format param to existing `/lineage` endpoint
- Pros: Fewer routes
- Cons: Changes behavior of existing endpoint, potential SDK breakage
- Risk: Medium — modifies existing contract

## Recommendation
Solution A — New `/documents/{document_id}/lineage/export` endpoint. Keeps existing API stable, supports `?format=json|csv` query parameter, returns downloadable file with proper Content-Disposition headers. CSV flattens hierarchical data to one-row-per-chunk for spreadsheet analysis.

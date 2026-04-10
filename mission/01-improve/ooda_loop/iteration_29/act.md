# OODA-29 Act

## Changes
- **intent.rs**: WHY comment on heuristic fallback + 7 edge case tests (from_str_loose all variants, unknown fallback, display roundtrip, default, empty query, case-insensitive, all recommended modes)
- **chunk_retrieval.rs**: WHY comment on first-seen-wins merge + 4 edge cases (empty, single, all duplicates, order preservation)

## Evidence
- Tests: 1314 → 1325 (+11)
- Clippy: 0 warnings

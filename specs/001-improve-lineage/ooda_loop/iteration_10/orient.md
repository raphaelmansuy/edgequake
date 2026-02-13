# Analysis - Iteration 10

## Solution
Update TypeScript types to match backend API enhancements, then update UI components to display them.

1. Add `document_type`, `sha256_checksum`, `page_count`, `file_size_bytes` to `Document` interface
2. Add `start_line`, `end_line` to `ChunkDetail` interface
3. Add `DocumentFullLineageResponse` and `ChunkLineageApiResponse` types
4. Update `SourceInfoGrid` to conditionally display new fields

## Risk: Low
All new fields are optional. Existing UI behavior unchanged when fields are absent.

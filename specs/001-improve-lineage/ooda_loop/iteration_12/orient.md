# Analysis - Iteration 12

## Current Gap
MetadataSidebar shows metadata only from the Document prop. KV storage holds richer metadata from ingestion (OODA-04/05/06) that isn't accessible via standard document fetch.

## Solution: EnhancedMetadata component
Create a new component that:
1. Fetches `/documents/:id/metadata` via `useDocumentMetadata` hook (OODA-11)
2. Filters out fields already displayed by other components (SourceInfoGrid, ProcessingDetails)
3. Renders remaining fields in a dynamic key-value grid
4. Handles arrays with Badge chips and long strings with truncation

## Benefits
- Zero-config: auto-discovers and displays all KV metadata fields
- Future-proof: new metadata fields added during ingestion automatically appear
- No overlap: SKIP_FIELDS set prevents duplication with existing components

# Decision - Iteration 29

## Changes to Make
1. Add WHY comments to 5 handlers: `get_chunk_detail`, `get_entity_provenance`,
   `get_entity_lineage`, `get_document_lineage`, `get_chunk_lineage`, `get_document_full_lineage`
2. Improve error messages in entity lookups to mention normalization behavior
3. Improve export handler error to mention processing requirement

## Expected Outcome
- Q3 (WHY comments) fully satisfied for lineage handlers
- Q4 (actionable errors) improved for entity and export endpoints
- T7 (clippy clean) maintained

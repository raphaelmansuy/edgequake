# OODA-41: Observe

## Observation

Documents can have metadata attached during upload. Need to verify:

1. Metadata is preserved during ingestion
2. Metadata doesn't affect deletion
3. Custom metadata fields work correctly

## Gap

No tests for metadata handling during add/delete lifecycle.

## Evidence

Looking for metadata-related fields in upload/delete responses.

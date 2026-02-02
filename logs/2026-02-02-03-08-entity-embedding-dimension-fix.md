# Task Log: Entity Embedding Dimension Mismatch Fix

**Date:** 2026-02-02 03:08 UTC  
**Duration:** ~10 minutes  
**Commit:** 272ffb81

## Actions

- Analyzed backend logs showing 264 entity embedding failures with "expected 768, got 1536" error
- Located bug in processor.rs line 1141: used `self.vector_storage` instead of `workspace_vector_storage`
- Fixed by changing entity embedding storage to use workspace-specific vector table
- Verified documents.rs already uses correct workspace-specific storage (no bug there)
- Confirmed build succeeds and no clippy warnings
- Committed fix with detailed explanation

## Decisions

- Single-line fix: changed `self.vector_storage` → `workspace_vector_storage` in entity embedding loop
- Added WHY comment explaining dimension mismatch prevention
- Kept error handling as warn-only to allow partial success (consistent with current pattern)

## Next Steps

- Monitor backend logs after restart to confirm entity embeddings store successfully
- Test with new document upload to verify 0 embedding storage failures
- Consider adding integration test for multi-dimensional workspace scenarios

## Lessons/Insights

- Root cause: Chunk embeddings used workspace storage, but entity embeddings used legacy global storage
- This created a dimension mismatch when workspace used OpenAI (1536) but global table had Ollama (768)
- Bug only affected processor.rs; documents.rs handlers were already correct
- Fix aligns entity storage with chunk storage pattern (both use workspace-specific tables)

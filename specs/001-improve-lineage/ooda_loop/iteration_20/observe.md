# Observation - Iteration 20

## Focus: Operations Debugging Guide for Metadata/Lineage

## Current State

- Documentation deliverables 1-3 complete (architecture, API reference, tutorial)
- Mission requires `docs/operations/metadata-debugging.md` for operators
- No existing debugging guide for lineage-specific issues

## Key Failure Modes Identified

1. Missing metadata fields (pre-enhancement documents)
2. Chunks without position data
3. Entity extraction failures (LLM unavailable)
4. Missing model names on chunks
5. Broken PDF → Document links
6. Lineage not persisted

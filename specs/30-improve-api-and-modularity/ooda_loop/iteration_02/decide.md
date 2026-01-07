# OODA Iteration 02 - Decide

**Date**: 2026-01-07
**Focus**: Implementation plan for helper integration

## Decision

Systematically replace all duplicated patterns in `sota_engine.rs` with helper functions.

## Implementation Plan

### Step 1: Chunk Patterns (5 instances)

- Find all `RetrievedChunk::new` calls
- Replace each with `build_chunk_from_result(result)`
- Run tests after each replacement

### Step 2: Entity Patterns (4 instances)

- Find all `RetrievedEntity::new` calls
- Replace each with `build_entity_from_node(id, props, degree, score)`
- Handle score parameter (use 0.0 for popularity-based, actual score for vector-based)

### Step 3: Relationship Patterns (3/4 instances)

- Find all `RetrievedRelationship::new` calls using graph edges
- Replace with `build_relationship_from_edge(source, target, props)`
- Keep vector-based pattern unchanged

### Step 4: Clean Up Imports

- Remove unused imports: `RetrievedChunk`, `RetrievedEntity`
- Remove unused helper imports: `extract_document_id`, `extract_entity_source_tracking`
- Add missing imports: `build_entity_from_node`

### Step 5: Validate

- Run full workspace tests
- Verify line count reduction

## Success Criteria

1. All tests pass (0 failures)
2. sota_engine.rs reduced by ~300+ lines
3. No functional changes

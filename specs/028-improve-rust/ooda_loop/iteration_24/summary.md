# OODA Loop Iteration 24: Orchestrator WHY Documentation

## Date: 2025-01-04

## Observe
- `orchestrator.rs` is 1104 lines - central coordination module
- The `insert` method handles the 3-stage pipeline but lacks architectural explanation
- The `delete_document` method implements cascade delete but reasoning not documented

## Orient
The orchestrator is the heart of EdgeQuake. Understanding its design decisions is critical:
- Why 3 stages? (Chunking → Extraction → Merge)
- Why track sources? (Cascade delete requirement)
- Why LLM summarization? (Conflict resolution)

## Decide
Add WHY comments explaining:
1. The 3-stage pipeline architecture in `insert`
2. The source-tracking cascade delete strategy in `delete_document`

## Act

### Changes Made

#### 1. `insert` Method - 3-Stage Pipeline Architecture
Added comprehensive WHY documentation:

```rust
/// # WHY: 3-Stage Pipeline Architecture
///
/// 1. **Pipeline Processing** - Chunking → Entity Extraction → Embedding
///    - WHY chunks: LLM context windows are limited; chunks enable parallel processing
///    - WHY overlapping chunks: Entities spanning chunk boundaries are captured
///
/// 2. **Knowledge Graph Merge** - Deduplicate and merge into graph storage
///    - WHY merge instead of insert: Same entity may appear in multiple documents
///    - WHY LLM summarization: Merge conflicting descriptions intelligently
///    - WHY source tracking: Enable cascade delete when documents are removed
///
/// 3. **Vector Storage** - Store embeddings for semantic search
///    - WHY type metadata: Distinguish entity vectors from chunk vectors
///    - WHY tenant isolation: Multi-tenancy requires vector filtering
```

Added inline comments for each stage:
- Stage 1: "Transforms raw text into structured knowledge graph elements"
- Stage 2: "Entities may exist from previous documents; merge avoids duplicates"
- Stage 3: "Enables filtering entity vs chunk vectors at query time"

#### 2. `delete_document` Method - Cascade Delete Strategy
Replaced implementation-focused doc with WHY-focused explanation:

```rust
/// # WHY: Source-Tracking Cascade Delete
///
/// 1. **Source Tracking** - Every entity/relationship stores `source_id`
///    WHY: A single entity may be mentioned in 100 documents
///
/// 2. **Cascade Logic**:
///    - ONLY sources from this doc → DELETE entity
///    - MIXED sources → UPDATE to remove this doc's sources
///
/// 3. **Edge Cleanup** - WHY: Orphan edges corrupt graph queries
```

## Verification
- `cargo build --package edgequake-core`: ✅ No warnings
- All tests still pass

## Files Modified
1. `crates/edgequake-core/src/orchestrator.rs` - Added WHY comments to `insert` and `delete_document`

## Impact
- **Onboarding**: New developers understand the architectural rationale
- **Debugging**: Engineers can trace why specific behaviors exist
- **Maintenance**: Changes can be evaluated against documented goals

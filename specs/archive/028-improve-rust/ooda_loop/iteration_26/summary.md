# OODA Loop Iteration 26: SOTA Query Engine WHY Documentation

## Date: 2025-01-04

## Observe

- `sota_engine.rs` is 1989 lines - the main query engine
- The 5-stage query pipeline lacks architectural explanation
- Mode-specific methods (query_local, query_global) lack WHY context

## Orient

The SOTA engine implements LightRAG's proven query architecture:

- 5-stage pipeline: Keywords → Validation → Mode → Retrieval → Budgeting
- Mode-specific strategies: Local (entities), Global (relationships), Hybrid (both)

## Decide

Add WHY comments to:

1. Main `query` method - explain the 5-stage pipeline
2. `query_local` - explain entity-centric strategy
3. `query_global` - explain relationship-centric strategy

## Act

### Changes Made

#### 1. `query` Method - 5-Stage Pipeline Documentation

```rust
/// # WHY: 5-Stage Query Pipeline
///
/// 1. **Keyword Extraction** - Extract high/low-level keywords using LLM
///    - WHY high-level: Relationships (e.g., "partnership", "acquired")
///    - WHY low-level: Entities (e.g., "Apple", "Microsoft")
///
/// 2. **Keyword Validation** - Check keywords exist in knowledge graph
///    - WHY: Non-existent keywords dilute embedding computation
///
/// 3. **Mode Selection** - Choose retrieval strategy
///    - Local: Entities + 1-hop neighbors
///    - Global: Relationships + community summaries
///
/// 4. **Vector Retrieval** - Semantic search with mode-specific embedding
///
/// 5. **Token Budgeting** - Fit context within LLM limits
```

#### 2. `query_local` Method - Entity-Centric Strategy

```rust
/// # WHY: Local Mode Strategy
///
/// Local mode answers specific factual questions:
/// 1. Low-level embedding uses entity-focused keywords
/// 2. Entity vector filter ignores relationships
/// 3. 1-hop graph expansion fetches neighbors
/// 4. Degree-based ranking prioritizes connected entities
```

#### 3. `query_global` Method - Relationship-Centric Strategy

```rust
/// # WHY: Global Mode Strategy
///
/// Global mode answers thematic/analytical questions:
/// 1. High-level embedding uses relationship keywords
/// 2. Relationship vector filter focuses on connections
/// 3. Entity hydration provides endpoint context
/// 4. Community summaries add thematic overview
```

## Verification

- `cargo build --package edgequake-query`: ✅ No warnings
- All tests still pass

## Files Modified

1. `crates/edgequake-query/src/sota_engine.rs` - Added WHY comments to query pipeline

## Impact

- **Algorithm Understanding**: Developers see why each step exists
- **Debugging**: Easier to trace which stage is causing issues
- **Mode Selection**: Clear when to use Local vs Global vs Hybrid

# OODA Iteration 122: Orient

## Date: 2026-01-14

## Analysis: Model Lineage & Token Display

### Current Data Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         QUERY RESPONSE DATA FLOW                             │
└─────────────────────────────────────────────────────────────────────────────┘

Engine QueryStats                     API QueryStats (exposed to client)
(edgequake-query)                     (edgequake-api)
─────────────────                     ─────────────────────────────────
├── embedding_time_ms      ─────────> ├── embedding_time_ms
├── retrieval_time_ms      ─────────> ├── retrieval_time_ms  
├── generation_time_ms     ─────────> ├── generation_time_ms
├── total_time_ms          ─────────> ├── total_time_ms
├── context_tokens         ──────X─── │ (NOT EXPOSED!)
├── generated_tokens       ──────X─── │ (NOT EXPOSED!)
└── (model info missing!)             └── sources_retrieved

MISSING DATA:
1. generated_tokens / tokens_used
2. tokens_per_second (calculated)
3. model provider (e.g., "ollama")
4. model name (e.g., "gemma3:12b")
```

### Gap Analysis

| Field | Source | Destination | Status |
|-------|--------|-------------|--------|
| `generated_tokens` | engine.stats | API response | ❌ Missing |
| `tokens_per_second` | calculated | API response | ❌ Missing |
| `model_provider` | workspace config | API response | ❌ Missing |
| `model_name` | workspace config | API response | ❌ Missing |

### Solution Design

**Option A: Extend QueryStats**

Add fields directly to existing QueryStats struct:
```rust
pub struct QueryStats {
    // ... existing fields ...
    pub tokens_used: Option<usize>,       // NEW
    pub tokens_per_second: Option<f32>,   // NEW
    pub model_provider: Option<String>,   // NEW
    pub model_name: Option<String>,       // NEW
}
```

**Option B: Add Separate ModelLineage**

Add a new struct for model info:
```rust
pub struct ModelLineage {
    pub provider: String,
    pub model: String,
    pub tokens_used: usize,
    pub tokens_per_second: f32,
}
```

**Recommendation**: Option A is simpler and keeps related stats together.

### Implementation Impact

Files to modify:
1. `edgequake-api/src/handlers/query_types.rs` - Add new fields to QueryStats
2. `edgequake-api/src/handlers/query.rs` - Populate new fields from engine result
3. Frontend components will need to display these new fields

### WebUI Display Consideration

Per spec Item 22: Display format should be `58.5/s • ollama/gemma3:12b`

```
┌─────────────────────────────────────────────────────────────────┐
│ Query: What is machine learning?                                │
├─────────────────────────────────────────────────────────────────┤
│ Answer text here...                                             │
│                                                                 │
│ ─────────────────────────────────────────────────────────────── │
│ Retrieval: 45ms | Generation: 1.2s | 124 tokens • 58.5/s        │
│ Model: ollama/gemma3:12b                                        │
└─────────────────────────────────────────────────────────────────┘
```

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| Breaking API change | New fields are optional, backward compatible |
| WebUI regression | Fields skip_serializing_if None |
| Performance overhead | Calculation is O(1), negligible |

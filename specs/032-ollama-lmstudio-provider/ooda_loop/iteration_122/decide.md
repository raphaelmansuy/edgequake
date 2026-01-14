# OODA Iteration 122: Decide

## Date: 2026-01-14

## Decision 1: Add Token & Model Info to QueryStats

**Decision**: Extend `QueryStats` in `query_types.rs` with new optional fields.

**Fields to Add**:
```rust
/// Number of tokens generated in the response.
#[serde(skip_serializing_if = "Option::is_none")]
pub tokens_used: Option<usize>,

/// Tokens per second generation speed.
#[serde(skip_serializing_if = "Option::is_none")]
pub tokens_per_second: Option<f32>,

/// LLM provider used for generation (e.g., "ollama", "openai").
#[serde(skip_serializing_if = "Option::is_none")]
pub llm_provider: Option<String>,

/// LLM model name used (e.g., "gemma3:12b", "gpt-4o-mini").
#[serde(skip_serializing_if = "Option::is_none")]
pub llm_model: Option<String>,
```

**Rationale**:
- Optional fields maintain backward compatibility
- `skip_serializing_if` prevents bloating responses for clients that don't need it
- Combined provider/model gives complete lineage

## Decision 2: Calculate Tokens Per Second

**Formula**: 
```rust
tokens_per_second = (tokens_used as f32) / (generation_time_ms as f32) * 1000.0
```

**Edge Cases**:
- If `generation_time_ms == 0`: return `None`
- If `tokens_used == 0`: return `Some(0.0)`

## Decision 3: Source Model Info from Request/Context

**Problem**: Query handler doesn't have direct access to which LLM model was used.

**Solution**: 
1. Check `request.provider` if specified by client
2. Otherwise, use workspace configuration
3. Fallback to server defaults

## Action Plan

```
Step 1: Add fields to QueryStats (query_types.rs)
        - tokens_used: Option<usize>
        - tokens_per_second: Option<f32>
        - llm_provider: Option<String>
        - llm_model: Option<String>

Step 2: Update execute_query (query.rs)
        - Map engine stats.generated_tokens to tokens_used
        - Calculate tokens_per_second
        - Add provider/model from workspace or defaults

Step 3: Run tests
        - Verify serialization works
        - Verify backward compatibility

Step 4: Document in OpenAPI schema
        - Fields are auto-documented via ToSchema derive
```

## Files to Modify

| File | Changes |
|------|---------|
| `edgequake-api/src/handlers/query_types.rs` | Add 4 new fields to QueryStats |
| `edgequake-api/src/handlers/query.rs` | Populate new fields |

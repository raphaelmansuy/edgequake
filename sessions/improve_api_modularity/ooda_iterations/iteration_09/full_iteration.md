# OODA Iteration 09 - Add validate_query() and Apply to Query Handlers

## Observe

- Analyzed `unwrap()` usage across codebase (581 total, mostly in tests/lock patterns)
- Identified duplicated validation pattern in query.rs (lines 185-194 and 418-427)
- Pattern: empty check + length check with identical error messages

## Orient

- DRY opportunity: extract query validation to validation.rs module
- Same pattern as validate_content() but for query strings
- Two handlers affected: `query_knowledge_graph()` and `query_streaming()`

## Decide

1. Create `validate_query(query, max_length)` function in validation.rs
2. Add unit tests for the new function
3. Apply to both handlers in query.rs
4. Verify non-regression

## Act

### Created validate_query() Function

```rust
pub fn validate_query(query: &str, max_length: usize) -> ApiResult<()> {
    if query.trim().is_empty() {
        return Err(ApiError::ValidationError("Query cannot be empty".to_string()));
    }
    if query.len() > max_length {
        return Err(ApiError::BadRequest(format!(
            "Query exceeds maximum length of {} characters", max_length
        )));
    }
    Ok(())
}
```

### Added 3 Tests

- `test_validate_query_success` - valid query passes
- `test_validate_query_empty` - empty/whitespace rejected
- `test_validate_query_too_long` - oversized query rejected

### Applied to query.rs

Before (duplicated 2x, 10 lines each):

```rust
if request.query.trim().is_empty() {
    return Err(ApiError::ValidationError("Query cannot be empty".to_string()));
}
if request.query.len() > state.config.max_query_length {
    return Err(ApiError::BadRequest(format!(
        "Query exceeds maximum length of {} characters",
        state.config.max_query_length
    )));
}
```

After (1 line each):

```rust
validate_query(&request.query, state.config.max_query_length)?;
```

## Metrics

| Metric               | Before | After | Change    |
| -------------------- | ------ | ----- | --------- |
| query.rs lines       | 519    | 504   | -15 (-3%) |
| validation.rs tests  | 11     | 14    | +3        |
| validation functions | 3      | 4     | +1        |

## Test Results

- validation module: 14/14 passed ✅
- edgequake-api lib: 108/108 passed ✅
- clippy: 0 warnings ✅

## Commit

`481467c` - refactor(api): Add validate_query() and apply to query handlers

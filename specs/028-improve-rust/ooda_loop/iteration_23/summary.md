# OODA Loop Iteration 23: Actionable Error Documentation

## Date: 2025-01-04

## Observe
- Errors often lack actionable guidance for users and operators
- Users hitting errors need to know: what went wrong, why, and how to fix it
- Unused variable warning in state.rs from conditional compilation

## Orient
Focus on two critical error modules:
1. **edgequake-llm/src/error.rs** - LLM provider errors need clear debugging guidance
2. **edgequake-api/src/error.rs** - HTTP errors need status code mapping documentation
3. **Fix unused variable warning** - Conditional compilation quirk

## Decide
1. Add error handling philosophy documentation to LLM errors
2. Add common errors/solutions table to help operators debug quickly
3. Add HTTP status code mapping with retry guidance for API errors
4. Fix `default_user_id` unused variable warning with proper annotation

## Act

### Changes Made

#### 1. edgequake-llm/src/error.rs - Error Handling Philosophy
Added comprehensive documentation explaining:
- Error categorization (transient vs permanent)
- Retry behavior recommendations
- Common errors table with causes and solutions

```rust
/// # Error Handling Philosophy
///
/// LLM errors are categorized by recoverability:
///
/// | Category | Examples | Retry? | Action |
/// |----------|----------|--------|--------|
/// | Transient | RateLimit, Timeout | Yes | Exponential backoff |
/// | Permanent | Authentication, InvalidModel | No | Fix configuration |
/// | Partial | TokenLimit exceeded | Maybe | Reduce input size |
```

#### 2. edgequake-api/src/error.rs - HTTP Status Mapping
Added detailed HTTP status code documentation:
- Maps each error variant to HTTP status (4xx/5xx)
- Explains retry semantics (Retry-After header, exponential backoff)
- Provides clear user actions for each error type

```rust
/// ## HTTP Status Code Mapping
///
/// | Error Variant | HTTP Status | Retry? | User Action |
/// |---------------|-------------|--------|-------------|
/// | NotFound | 404 | No | Verify resource ID exists |
/// | Unauthorized | 401 | No | Check API key |
/// | RateLimited | 429 | Yes | Wait, then retry |
```

#### 3. edgequake-api/src/state.rs - Fixed Unused Variable Warning
```rust
// WHY: Variable is only used when postgres feature is enabled
// Using _ prefix to suppress unused warning in non-postgres builds
let _default_user_id = self.config.default_user_id.clone();
```

## Verification
- `cargo build --package edgequake-llm`: ✅ No warnings
- `cargo build --package edgequake-api`: ✅ No warnings
- All tests still pass

## Files Modified
1. `crates/edgequake-llm/src/error.rs` - Added error handling philosophy and common errors table
2. `crates/edgequake-api/src/error.rs` - Added HTTP status code mapping documentation
3. `crates/edgequake-api/src/state.rs` - Fixed unused variable with `_` prefix

## Impact
- **Developer Experience**: Error documentation guides debugging decisions
- **Operator Experience**: Clear table helps diagnose production issues quickly
- **Code Quality**: Fixed clippy warning, clean builds maintained

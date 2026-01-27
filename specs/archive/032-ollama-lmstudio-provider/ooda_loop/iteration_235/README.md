# OODA-235: Embedding Provider Resolution Duplication Analysis

## Observe

Found two implementations of embedding provider resolution:

### Location 1: `providers/resolver.rs` (lines 274-324)

```rust
pub async fn resolve_embedding_provider(
    &self,
    workspace_id: &str,
) -> Result<ResolvedEmbeddingProvider, ProviderResolutionError>
```

### Location 2: `handlers/query.rs` (lines 528-602)

```rust
pub async fn get_workspace_embedding_provider(
    state: &AppState,
    workspace_id: &str,
) -> Result<Option<Arc<dyn EmbeddingProvider>>, ApiError>
```

### Differences

| Aspect                  | `resolver.rs`                                                | `query.rs`                                             |
| ----------------------- | ------------------------------------------------------------ | ------------------------------------------------------ |
| Return type             | `Result<ResolvedEmbeddingProvider, ProviderResolutionError>` | `Result<Option<Arc<dyn EmbeddingProvider>>, ApiError>` |
| Empty provider handling | Returns error                                                | Returns `Ok(None)`                                     |
| Error messages          | Generic                                                      | Detailed API key guidance                              |
| Additional info         | Returns dimension, model name                                | Just returns provider                                  |
| Called from             | Nowhere yet                                                  | chat.rs, query.rs handlers                             |

### Usage Pattern

```
chat.rs:422 → get_workspace_embedding_provider
chat.rs:859 → get_workspace_embedding_provider
query.rs:181 → get_workspace_embedding_provider
```

## Orient

### Why This Duplication Exists

1. **Different semantics**: resolver returns error on empty, handler returns `None` for fallback
2. **Different error types**: `ProviderResolutionError` vs `ApiError`
3. **History**: resolver was added in OODA-226 but handlers weren't refactored

### Risk Assessment

| Risk                        | Severity | Mitigation                                          |
| --------------------------- | -------- | --------------------------------------------------- |
| Logic divergence            | MEDIUM   | Will cause bugs if one is updated but not the other |
| Error message inconsistency | LOW      | User confusion but not functional                   |
| Missed safety features      | HIGH     | If resolver adds timeout, query.rs won't get it     |

### Refactoring Strategy

**Option A: Wrapper Function**

- Keep query.rs function but have it delegate to resolver
- Handle `None` case and error conversion in wrapper

**Option B: Unified Interface**

- Change resolver to return `Option<ResolvedEmbeddingProvider>`
- Update all callers

**Option C: Accept Duplication**

- Document as technical debt
- Add tests to catch divergence

## Decide

**Action**: Option C (Accept Duplication) for now

**Rationale**:

1. Both implementations are functionally correct
2. Both use `create_safe_embedding_provider` (safety limit already in place)
3. Refactoring would touch 4+ files and require extensive testing
4. Lower priority than other OODA loops

**Future Work**:

- Track as OODA-250 for future consolidation
- Add cross-reference comments to both implementations

## Act

Added cross-reference comments to both implementations:

### resolver.rs

```rust
/// NOTE: Similar logic exists in handlers/query.rs::get_workspace_embedding_provider
/// See OODA-235 for duplication analysis
```

### query.rs

```rust
/// NOTE: Similar logic exists in providers/resolver.rs::resolve_embedding_provider
/// See OODA-235 for duplication analysis
```

## Metrics

| Metric            | Value                 |
| ----------------- | --------------------- |
| Duplicated lines  | ~50                   |
| Affected handlers | 2 (chat.rs, query.rs) |
| Call sites        | 3                     |
| Risk level        | MEDIUM                |
| Priority          | DEFER                 |

## Next Steps

- OODA-250: Consolidate embedding provider resolution (future)
- Continue with higher-priority OODA loops

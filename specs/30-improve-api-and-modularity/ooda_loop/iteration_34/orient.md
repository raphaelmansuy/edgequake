# Iteration 34 - Orient

**Date:** 2026-01-08  
**Focus:** documents.rs modularization strategy

## Analysis

### Why Modularize?

1. **SRP Violation:** 2,903 lines with 13 functions covering 6 distinct concerns
2. **Cognitive Load:** Hard to navigate and understand the full file
3. **Testing:** Monolithic file makes targeted testing harder
4. **Maintainability:** Changes risk unintended side effects

### Dependencies Analysis

```
documents.rs imports:
├── axum (State, Json, Multipart)
├── crate::error (ApiError, ApiResult)
├── crate::middleware (TenantContext)
├── crate::state (AppState)
└── crate::handlers::documents_types::*
```

All handlers share the same dependency pattern - no circular dependencies.

### Modularization Trade-offs

| Approach           | Pros                             | Cons                              |
| ------------------ | -------------------------------- | --------------------------------- |
| Split by operation | Clear separation, easier testing | More files to maintain            |
| Keep monolithic    | Simpler structure                | SRP violation, hard to navigate   |
| Service layer      | Maximum decoupling               | Over-engineering for current size |

### Recommended Strategy

Create a `documents/` directory with submodules, but maintain a single public interface.

**Reasoning:**

- Existing pattern in the codebase (streaming/, handlers/)
- Backward compatible - can keep re-exports in documents.rs
- Incremental migration possible

## Key Insight

The spec mentions extracting `postgres_workspace_service.rs` into a crate. However, these services already:

1. Implement core traits (`WorkspaceService`, `ConversationService`)
2. Are well-encapsulated
3. Have clear single responsibility

**Decision:** Focus on handler modularization first, service layer extraction is lower priority.

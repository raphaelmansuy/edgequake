# OODA Iteration 102: Architecture Analysis - Service Layer Placement

**Date**: 2025-01-06
**Focus**: Analyze trade-off of moving PostgreSQL services from Core to Storage crate

## Observe

User asked: "What about having a stub of service in core and moving the storage part in edgequake-storage?"

Current Architecture:

```
edgequake-storage (no dep on core)
├── PostgresConversationStorage    ← Raw DB operations
└── ConversationRow                ← DB row types

edgequake-core (depends on storage)
├── ConversationService trait      ← Port (abstraction)
├── PostgresConversationService    ← Adapter (wraps storage)
├── InMemoryConversationService    ← Adapter (test/dev)
└── Conversation                   ← Domain type
```

## Orient

**Proposed Change**: Move `PostgresConversationService` and `PostgresWorkspaceService` to `edgequake-storage`

**Analysis**:

### Option A: Keep in Core (Current) ✅ CHOSEN

- Services implement core traits → live next to traits
- No circular dependency
- Follows Hexagonal Architecture: Adapters can live with Ports

### Option B: Move to Storage ❌ BLOCKED

- Would require storage → core dependency (for trait)
- But core already → storage dependency (for storage types)
- **CIRCULAR DEPENDENCY BLOCKER**

### Option C: Create edgequake-services crate

- Could work but adds complexity
- One more crate to maintain
- Not worth it for 2 services

### Option D: Create edgequake-traits crate

- Break traits out of core
- storage → traits, core → traits, core → storage
- Over-engineering for current needs

## Decide

**Decision: Keep current architecture (Option A)**

The current design is correct because:

1. **Rust Orphan Rule**: Cannot implement foreign trait for foreign type
2. **Circular dependency prevention**: Storage cannot depend on Core
3. **Hexagonal Architecture**: Adapters can co-locate with Ports
4. **Pragmatic**: Two services don't justify new crate

## Act

1. ✅ Updated WHY comments in both service files explaining the BLOCKER
2. ✅ No code changes needed - architecture is correct
3. ✅ Documentation added for future developers

## Files Modified

- `edgequake-core/src/postgres_conversation_service.rs` - Enhanced WHY comment
- `edgequake-core/src/postgres_workspace_service.rs` - Enhanced WHY comment

## Outcome

**Analysis-only iteration** - Confirmed current architecture is optimal.
Current design follows DDD, Hexagonal Architecture, and Rust idioms correctly.

## Key Insight

The two-layer design is **intentional**:

- Storage layer: Raw database operations, returns `ConversationRow`
- Service layer: Domain adapter, converts to `Conversation` domain type

This separation is a feature, not a bug.

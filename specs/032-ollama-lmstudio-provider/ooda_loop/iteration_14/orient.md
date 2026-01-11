# OODA Loop Iteration 14 - Orient Phase

**Date:** 2026-01-11  
**Focus:** Design for Tenant-Level Model Configuration  
**Status:** ✅ COMPLETE

## Architecture Analysis

### Hierarchical Model Configuration

```
┌────────────────────────────────────────────────────────────────┐
│                         TENANT                                  │
│  ┌────────────────────────────────────────────────────────────┐│
│  │  Default LLM: ollama/gemma3:12b                            ││
│  │  Default Embedding: openai/text-embedding-3-small (1536)   ││
│  └────────────────────────────────────────────────────────────┘│
│                              │                                  │
│                              ▼ (inherits if not specified)      │
│  ┌─────────────────┐    ┌─────────────────┐                    │
│  │   Workspace A   │    │   Workspace B   │                    │
│  │ LLM: inherited  │    │ LLM: gpt-4o     │ (override)         │
│  │ Embed: inherited│    │ Embed: inherited│                    │
│  └─────────────────┘    └─────────────────┘                    │
└────────────────────────────────────────────────────────────────┘
```

### Data Flow

1. **Tenant Creation:**
   - User specifies default LLM and embedding config (optional)
   - If not provided, use server defaults from environment

2. **Workspace Creation:**
   - User specifies workspace-specific config (optional)
   - If not provided, inherit from parent tenant
   - Auto-created default workspace inherits from tenant

3. **Query/Ingestion:**
   - Use workspace config (always set, either explicit or inherited)

### Pattern Reuse

The `Workspace` struct already has model configuration fields. The same pattern applies to `Tenant`:

```rust
// Existing pattern in Workspace
pub struct Workspace {
    // ... basic fields ...
    pub llm_model: String,
    pub llm_provider: String,
    pub embedding_model: String,
    pub embedding_provider: String,
    pub embedding_dimension: usize,
}

// Applied to Tenant (with "default_" prefix)
pub struct Tenant {
    // ... basic fields ...
    pub default_llm_model: String,
    pub default_llm_provider: String,
    pub default_embedding_model: String,
    pub default_embedding_provider: String,
    pub default_embedding_dimension: usize,
}
```

### Storage Considerations

- **In-Memory:** Uses `Tenant::new()` which will auto-populate defaults
- **PostgreSQL:** `TenantRow::into_tenant()` extracts from metadata JSONB
- **Backward compatibility:** Existing tenants get server defaults if metadata missing

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Field naming | `default_*` prefix | Distinguishes from workspace config |
| Builder methods | `with_llm_config()`, `with_embedding_config()` | Consistent with Workspace pattern |
| Inheritance | At create_workspace handler | Clean separation of concerns |
| Response format | Include `*_full_id` fields | Matches WorkspaceResponse pattern |

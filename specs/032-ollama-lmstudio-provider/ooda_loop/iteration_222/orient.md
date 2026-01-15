# OODA Iteration 222 - Orient Phase

**Date:** 2026-01-15
**Focus:** Root Cause Analysis

## Root Cause Identification

### Primary Cause: Storage Mode Difference

The dimension mismatch error was occurring when:
1. Backend was running in **memory storage mode**
2. User changed workspace embedding model from OpenAI (1536-dim) to Ollama nomic-embed-text (768-dim)
3. In-memory vectors retained old 1536-dim embeddings
4. New queries generated 768-dim embeddings
5. Memory storage's strict validation rejected the dimension mismatch

### Why PostgreSQL Doesn't Have This Issue

PostgreSQL storage uses **workspace-specific vector tables**:
- Each workspace has its own vector table (e.g., `eq_eq_default_ws_b86bb135_vectors`)
- Table dimension is determined at creation time based on workspace embedding config
- Query uses workspace's configured embedding dimension

Backend log confirms proper dimension handling:
```
Creating workspace-specific embedding provider workspace_id=b86bb135 
  embedding_provider=openai 
  embedding_model=text-embedding-3-small 
  embedding_dimension=1536

Getting workspace-specific vector storage workspace_id=b86bb135 dimension=1536
```

## Context Analysis

### Why User Saw the Error

User was likely running with:
1. Memory storage mode (no DATABASE_URL set)
2. Changed workspace embedding configuration
3. Didn't rebuild embeddings after configuration change

### Current State

- Backend: PostgreSQL storage mode ✅
- Workspaces: Using workspace-specific vector tables ✅
- Dimension handling: Workspace-level embedding dimension respected ✅

## Technical Architecture

```
Memory Storage Mode:
┌─────────────┐    ┌─────────────────────┐
│ Query (768) │───▶│ Memory Store (1536) │ ❌ MISMATCH
└─────────────┘    └─────────────────────┘

PostgreSQL Mode:
┌─────────────────┐    ┌─────────────────────────────┐
│ Query (1536)    │───▶│ Workspace Table (1536)      │ ✅ MATCH
│ Uses workspace  │    │ eq_eq_default_ws_xxx_vectors│
│ embedding config│    └─────────────────────────────┘
└─────────────────┘
```

## Impact Assessment

| Scenario | Risk | Mitigation |
|----------|------|------------|
| Memory mode + embedding change | HIGH | Rebuild embeddings required |
| PostgreSQL mode | LOW | Workspace isolation handles dimensions |
| Cross-workspace queries | LOW | Each workspace uses own dimension |

## Resolution Path

The issue is resolved by:
1. Running backend with PostgreSQL storage (current state)
2. Workspace-specific vector tables handle dimensions correctly
3. Each workspace maintains its own embedding configuration and vectors

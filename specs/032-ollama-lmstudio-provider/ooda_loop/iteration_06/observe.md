# OODA Loop Iteration #06 - Observe Phase

**Date:** 2026-01-11  
**Mission:** Gap Analysis & Remaining Implementation Requirements  
**Phase:** Observe (Deep Code Analysis & Mission Alignment)

---

## Executive Summary

**Objective:** Analyze previous iterations (01-05), identify missing requirements from mission spec, and plan remaining 44 OODA loops (06-50) for full Ollama/LM Studio provider integration.

**Previous Work Summary:**

- ✅ Iterations 01-04: Backend provider infrastructure (factory, auto-detection)
- ✅ Iteration 05: Provider status UI in settings page
- ❌ **CRITICAL GAPS IDENTIFIED**: 15+ major requirements still missing

---

## 1. Mission Requirements Analysis

### 1.1 Completed Requirements (Iterations 01-05)

| Requirement                 | Status      | Files                                                                                                             | Notes                                                   |
| --------------------------- | ----------- | ----------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------- |
| **Ollama Provider**         | ✅ Complete | [`ollama.rs`](../../edgequake/crates/edgequake-llm/src/providers/ollama.rs) (551 lines)                           | Builder pattern, env config, 768-dim embeddings         |
| **LM Studio Detection**     | ✅ Partial  | [`factory.rs`](../../edgequake/crates/edgequake-llm/src/factory.rs) (364 lines)                                   | Uses OpenAI wrapper, not dedicated impl                 |
| **Provider Auto-Detection** | ✅ Complete | [`factory.rs#L100-L138`](../../edgequake/crates/edgequake-llm/src/factory.rs#L100-L138)                           | Priority: Ollama → OpenAI → Mock                        |
| **Environment Variables**   | ✅ Complete | Multiple files                                                                                                    | `OLLAMA_HOST`, `OLLAMA_MODEL`, `EDGEQUAKE_LLM_PROVIDER` |
| **Provider Status API**     | ✅ Complete | [`provider_types.rs`](../../edgequake/crates/edgequake-api/src/provider_types.rs) (173 lines)                     | `/api/v1/settings/provider/status` endpoint             |
| **Provider Status UI**      | ✅ Complete | [`provider-status-card.tsx`](../../edgequake_webui/src/components/settings/provider-status-card.tsx) (300+ lines) | Settings page only, auto-refresh 30s                    |

### 1.2 MISSING Requirements (Critical for Mission Success)

#### **Category A: Provider Implementation**

| ID     | Requirement                        | Severity    | Current State                        | Impact                                       |
| ------ | ---------------------------------- | ----------- | ------------------------------------ | -------------------------------------------- |
| **A1** | **Dedicated LM Studio Provider**   | 🔴 CRITICAL | Using OpenAI wrapper instead         | Cannot configure LM Studio-specific features |
| **A2** | **LM Studio Embedding Dimensions** | 🔴 CRITICAL | Hardcoded 1536, no verification      | Wrong dimensions = query failures            |
| **A3** | **Provider Health Checks**         | 🟡 HIGH     | Status API exists but no health ping | Cannot detect offline providers before query |

#### **Category B: WebUI Integration** (Mission Spec Priority)

| ID     | Requirement                           | Severity    | Spec Quote                                                                  | Current State      |
| ------ | ------------------------------------- | ----------- | --------------------------------------------------------------------------- | ------------------ |
| **B1** | **Query Interface Provider Selector** | 🔴 CRITICAL | "easy way to change provider on the query dialogue with selection dropdown" | ❌ Not implemented |
| **B2** | **Provider + Model Dropdown**         | 🔴 CRITICAL | "organized by provider and model in the chat query input"                   | ❌ Not implemented |
| **B3** | **Dynamic Provider Switching**        | 🔴 CRITICAL | "minimal disruption to existing workflows"                                  | ❌ Not implemented |

**Evidence:** Current [`query/page.tsx`](<../../edgequake_webui/src/app/(dashboard)/query/page.tsx>) has NO provider selector (only query mode dropdown for local/global/hybrid).

#### **Category C: Workspace-Level Embedding Configuration**

| ID     | Requirement                             | Severity    | Spec Quote                                                                      | Current State              |
| ------ | --------------------------------------- | ----------- | ------------------------------------------------------------------------------- | -------------------------- |
| **C1** | **Workspace Embedding Model Selection** | 🔴 CRITICAL | "Embedding is chosen at the workspace level"                                    | ❌ Not in workspace schema |
| **C2** | **Workspace Creation UI**               | 🔴 CRITICAL | "choose the embedding model when creating a new workspace"                      | ❌ No embedding selector   |
| **C3** | **Default Embedding Model Config**      | 🟡 HIGH     | "By default, we will use the default embedding model configured for the server" | ❌ No server-level default |
| **C4** | **Workspace-to-Embedding Persistence**  | 🔴 CRITICAL | Database must store embedding model per workspace                               | ❌ Not in schema           |

**Evidence:** Checked [`workspaces_types.rs`](../../edgequake/crates/edgequake-api/src/handlers/workspaces_types.rs), no `embedding_model` field in `CreateWorkspaceApiRequest` or `WorkspaceResponse`.

#### **Category D: Vector Database Recreation**

| ID     | Requirement                       | Severity    | Spec Quote                                                                 | Current State            |
| ------ | --------------------------------- | ----------- | -------------------------------------------------------------------------- | ------------------------ |
| **D1** | **Embedding Change Detection**    | 🔴 CRITICAL | "changing the embedding model will require recreating the vector database" | ❌ No detection logic    |
| **D2** | **Vector DB Wipe & Rebuild**      | 🔴 CRITICAL | "provide a mechanism to recreate the vector database"                      | ❌ No API endpoint       |
| **D3** | **Postgres AGE Recreation**       | 🔴 CRITICAL | Works with Postgres AGE storage                                            | ❌ Not implemented       |
| **D4** | **In-Memory Recreation**          | 🟡 HIGH     | Works with in-memory storage                                               | ❌ Not implemented       |
| **D5** | **Recreation Progress UI**        | 🟡 HIGH     | "minimize downtime"                                                        | ❌ No progress indicator |
| **D6** | **Edge Case: Empty Vector DB**    | 🟡 HIGH     | "handle gracefully"                                                        | ❌ No special handling   |
| **D7** | **Edge Case: Concurrent Queries** | 🔴 CRITICAL | "edge case must be handled gracefully"                                     | ❌ No query locking      |

#### **Category E: Query Process Alignment**

| ID     | Requirement                        | Severity    | Spec Quote                                                                                          | Current State                |
| ------ | ---------------------------------- | ----------- | --------------------------------------------------------------------------------------------------- | ---------------------------- |
| **E1** | **Query Uses Workspace Embedding** | 🔴 CRITICAL | "use the embedding model associated with the workspace to generate embeddings for incoming queries" | ❌ Uses global provider      |
| **E2** | **Dimension Mismatch Detection**   | 🟡 HIGH     | Provider status shows dimension mismatch                                                            | ✅ Partial (UI warning only) |
| **E3** | **Cross-Provider Query Testing**   | 🟡 HIGH     | Test OpenAI query → Ollama workspace                                                                | ❌ No tests                  |

#### **Category F: Documentation & Testing**

| ID     | Requirement                    | Severity    | Spec Quote                                                    | Current State             |
| ------ | ------------------------------ | ----------- | ------------------------------------------------------------- | ------------------------- |
| **F1** | **Setup Guides Per Provider**  | 🟡 HIGH     | "clear instructions on how to set up and use"                 | ❌ Not written            |
| **F2** | **Non-Regression Tests**       | 🔴 CRITICAL | "Non regression is your North Star"                           | ❌ No comprehensive suite |
| **F3** | **Postgres vs Memory Testing** | 🔴 CRITICAL | "must test for Postgres and in Memory storage backends"       | ❌ Not tested             |
| **F4** | **WebUI API Compatibility**    | 🔴 CRITICAL | "test the edgequake_webui to ensure no regression in the API" | ❌ No E2E tests           |

---

## 2. Code Architecture Analysis

### 2.1 Current Provider System

**File:** [`factory.rs`](../../edgequake/crates/edgequake-llm/src/factory.rs)

```rust
// Current LM Studio implementation (PROBLEM: Not dedicated)
fn create_lmstudio() -> Result<(Arc<dyn LLMProvider>, Arc<dyn EmbeddingProvider>)> {
    let host = std::env::var("LMSTUDIO_HOST")
        .unwrap_or_else(|_| "http://localhost:1234".to_string());

    // ISSUE: Uses OpenAI provider with compatibility wrapper
    // Cannot access LM Studio-specific features
    let provider = Arc::new(OpenAIProvider::compatible("lmstudio-key", base_url)
        .with_model(&model)
        .with_embedding_model(&embedding_model)
        .with_embedding_dimension(embedding_dim));

    Ok((provider.clone(), provider))
}
```

**Problem:** LM Studio has unique API behaviors (e.g., model listing, specific error codes) not available through OpenAI wrapper.

### 2.2 Workspace Schema Gap

**File:** [`workspaces_types.rs`](../../edgequake/crates/edgequake-api/src/handlers/workspaces_types.rs#L42-L51)

```rust
// Current CreateWorkspaceApiRequest (MISSING embedding_model field)
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateWorkspaceApiRequest {
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub max_documents: Option<usize>,
    // ❌ MISSING: pub embedding_model: Option<String>,
    // ❌ MISSING: pub embedding_dimension: Option<usize>,
}
```

**Required Changes:**

1. Add `embedding_model` field to workspace creation request
2. Add `embedding_model` and `embedding_dimension` to workspace response
3. Update database schema to store embedding configuration per workspace
4. Migrate existing workspaces with default embedding model

### 2.3 Query Flow Analysis

**File:** [`edgequake-query`](../../edgequake/crates/edgequake-query/src/) (need to investigate)

**Current Assumption:** Query uses global LLM/embedding provider from AppState.

**Required Behavior:**

1. Query handler receives workspace_id from headers
2. Lookup workspace embedding model from database
3. Create embedding provider instance with correct model
4. Generate query embeddings with workspace-specific provider
5. Ensure dimension matches vector DB stored embeddings

---

## 3. Critical Findings

### 3.1 Mission Alignment Score: **35% Complete**

**Completed (35%):**

- ✅ Backend provider infrastructure
- ✅ Provider status display (settings page only)
- ✅ Basic Ollama support

**Missing (65%):**

- ❌ WebUI query interface provider selector (mission priority #1)
- ❌ Workspace-level embedding configuration (mission priority #2)
- ❌ Vector database recreation (mission priority #3)
- ❌ Query process alignment with workspace embeddings
- ❌ Comprehensive testing (non-regression requirement)
- ❌ Documentation for all providers

### 3.2 High-Impact Missing Features

1. **Query Interface Provider Selector** (Mission Spec Line 33-34)

   - User cannot switch providers during query
   - No UI to see available models per provider
   - Must restart backend to change provider

2. **Workspace Embedding Isolation** (Mission Spec Line 42-48)

   - All workspaces share same embedding model
   - Cannot mix OpenAI workspace + Ollama workspace in same server
   - Database schema does not support per-workspace embeddings

3. **Vector DB Recreation** (Mission Spec Line 50-55)
   - Changing embedding dimensions = broken queries
   - No way to rebuild embeddings without manual database reset
   - Concurrent query handling undefined during recreation

---

## 4. Dependencies & Blockers

### 4.1 Technical Dependencies

| Component                | Status                       | Blocker For | Notes                                |
| ------------------------ | ---------------------------- | ----------- | ------------------------------------ |
| **Workspace Schema**     | ❌ Missing `embedding_model` | C1-C4, E1   | Requires DB migration                |
| **Query Engine**         | ❓ Unknown                   | E1-E3       | Need to read `edgequake-query` crate |
| **Storage Backend APIs** | ❓ Unknown                   | D3-D4       | Need vector clear/rebuild methods    |

### 4.2 Implementation Sequence (Cannot Parallelize)

```
1. Workspace Schema Update (C4) ← Must come FIRST
   ↓
2. Workspace Creation UI (C2)
   ↓
3. Query Engine Workspace Lookup (E1)
   ↓
4. Vector DB Recreation Logic (D1-D4)
   ↓
5. WebUI Provider Selector (B1-B3)
   ↓
6. Non-Regression Testing (F1-F4)
```

**WHY Sequential:** Workspace schema is foundation for all other features.

---

## 5. Scratchpad & Next Steps

### 5.1 Questions to Answer (Orient Phase)

1. **Storage Backend API:** Does Postgres/Memory storage have `clear_vectors()` method?
2. **Query Engine:** Where does query embedding generation happen?
3. **Workspace Service:** Does it support custom fields or need schema migration?
4. **LM Studio Models:** What are actual default models? (Spec says "gemma-3n-e4b-it-mlxmodel" - is this real?)

### 5.2 Immediate Actions (Decide Phase)

- [ ] Read `edgequake-query` crate to understand query flow
- [ ] Check `edgequake-storage` for vector clear/rebuild APIs
- [ ] Investigate workspace service schema extensibility
- [ ] Research LM Studio actual model names
- [ ] Plan database migration strategy for existing workspaces

### 5.3 OODA Loop Allocation (Remaining 44 Loops)

**Iterations 06-10:** Deep code analysis & architecture planning (CURRENT)  
**Iterations 11-15:** LM Studio dedicated provider + health checks  
**Iterations 16-20:** Workspace schema + embedding configuration  
**Iterations 21-25:** Vector DB recreation (Postgres + Memory)  
**Iterations 26-30:** Query engine workspace alignment  
**Iterations 31-35:** WebUI provider selector in query interface  
**Iterations 36-40:** Edge cases & error handling  
**Iterations 41-45:** Comprehensive testing (Postgres + Memory)  
**Iterations 46-48:** Documentation & setup guides  
**Iterations 49-50:** Final non-regression validation

---

## 6. Risk Assessment

### 6.1 High-Risk Items

| Risk                            | Impact  | Mitigation                                           |
| ------------------------------- | ------- | ---------------------------------------------------- |
| **Breaking API Changes**        | 🔴 HIGH | Add new endpoints, deprecate old ones gradually      |
| **Database Migration**          | 🔴 HIGH | Test with copy of prod data, provide rollback script |
| **Concurrent Query Corruption** | 🔴 HIGH | Implement query locking during vector recreation     |

### 6.2 Mitigation Strategies

1. **API Compatibility:** Keep old endpoints working, add versioning
2. **Feature Flags:** Gate new features behind environment variables
3. **Rollback Plan:** Document steps to revert schema changes

---

## 7. Files Requiring Changes (Initial Estimate)

### Backend (Rust)

- `edgequake-llm/src/providers/lmstudio.rs` (NEW, ~500 lines)
- `edgequake-core/src/workspace_service.rs` (+100 lines)
- `edgequake-core/src/workspace_service_impl.rs` (+150 lines)
- `edgequake-api/src/handlers/workspaces_types.rs` (+20 lines)
- `edgequake-api/src/handlers/workspaces.rs` (+50 lines)
- `edgequake-api/src/handlers/vector_rebuild.rs` (NEW, ~300 lines)
- `edgequake-query/src/query_engine.rs` (+100 lines)
- `edgequake-storage/src/traits.rs` (+30 lines)
- `edgequake-storage/src/memory.rs` (+80 lines)
- `edgequake-storage/src/postgres.rs` (+120 lines)

### Frontend (TypeScript/React)

- `edgequake_webui/src/types/workspace.ts` (+15 lines)
- `edgequake_webui/src/components/query/provider-selector.tsx` (NEW, ~250 lines)
- `edgequake_webui/src/components/workspace/embedding-selector.tsx` (NEW, ~200 lines)
- `edgequake_webui/src/app/(dashboard)/query/page.tsx` (+50 lines)
- `edgequake_webui/src/app/(dashboard)/workspaces/create/page.tsx` (+80 lines)

### Tests (Rust)

- `edgequake-llm/tests/lmstudio_provider.rs` (NEW, ~300 lines)
- `edgequake-api/tests/e2e_workspace_embedding.rs` (NEW, ~400 lines)
- `edgequake-api/tests/e2e_vector_rebuild.rs` (NEW, ~500 lines)
- `edgequake-query/tests/workspace_embedding.rs` (NEW, ~350 lines)

**Total Estimated Lines:** ~3,500 new + ~500 modified = **4,000 lines of code**

---

## 8. Conclusion

**Status:** Mission 35% complete after 5 iterations.  
**Critical Path:** Workspace schema → Query alignment → UI integration  
**Highest Priority:** Workspace embedding configuration (blocks everything else)

**Next Iteration (07):** Orient phase - Deep dive into query engine and storage APIs to understand implementation requirements.

---

**Commit Message for Iteration 06:**

```
docs(ooda-06): Comprehensive gap analysis for Ollama/LM Studio integration

- Identified 15+ missing requirements from mission spec
- Analyzed current architecture (35% complete)
- Documented workspace schema gaps (critical blocker)
- Mapped dependencies between features
- Planned remaining 44 OODA loops with clear milestones

Critical findings:
- Query interface provider selector missing (mission priority #1)
- Workspace-level embedding config not implemented (priority #2)
- Vector DB recreation not implemented (priority #3)
- No non-regression testing for storage backends

Files analyzed:
- edgequake-llm/src/factory.rs (364 lines)
- edgequake-llm/src/providers/ollama.rs (551 lines)
- edgequake-api/src/handlers/workspaces_types.rs (336 lines)
- edgequake-api/src/handlers/workspaces.rs (767 lines)

Estimated work: ~4,000 LOC across 20+ files
Next: Orient phase (iteration 07) - Query engine analysis
```

# OODA Iteration 08: Orient

**Date:** 2026-01-11  
**Focus:** Design LLM Provider Configuration for Workspaces

## Analysis

### Why Workspace-Level LLM Provider Matters

1. **Knowledge Graph Generation**: Uses LLM to extract entities/relationships from documents
2. **Document Summarization**: Uses LLM to create document summaries
3. **Entity Description Generation**: Uses LLM to generate entity descriptions
4. **Query-Time LLM**: Can be DIFFERENT from workspace LLM (user selects at query time)

```
┌─────────────────────────────────────────────────────────────────────┐
│                         WORKSPACE CONFIGURATION                      │
├─────────────────────────────────────────────────────────────────────┤
│                                                                       │
│  ┌─────────────────────────┐    ┌─────────────────────────┐         │
│  │   INGESTION PIPELINE    │    │     QUERY PIPELINE      │         │
│  ├─────────────────────────┤    ├─────────────────────────┤         │
│  │                         │    │                         │         │
│  │  LLM Provider: ollama   │    │  LLM: User's Choice     │         │
│  │  LLM Model: gemma3:12b  │    │  (from UI dropdown)     │         │
│  │                         │    │                         │         │
│  │  Used for:              │    │  Used for:              │         │
│  │  • Entity extraction    │    │  • Answer generation    │         │
│  │  • Relationship mining  │    │  • Response streaming   │         │
│  │  • Summarization        │    │                         │         │
│  │                         │    │                         │         │
│  ├─────────────────────────┤    ├─────────────────────────┤         │
│  │                         │    │                         │         │
│  │  Embedding Provider:    │    │  Embedding Provider:    │         │
│  │  ollama                 │    │  SAME AS INGESTION!     │         │
│  │  embeddinggemma:768     │    │  (must match vectors)   │         │
│  │                         │    │                         │         │
│  └─────────────────────────┘    └─────────────────────────┘         │
│                                                                       │
└─────────────────────────────────────────────────────────────────────┘
```

### Provider/Model ID Format Decision

The spec requires `provider/model_name` format. Options:

**Option A: Combined String**

```rust
pub llm_model_id: String,  // "ollama/gemma3:12b"
```

- Pros: Single field, matches spec exactly
- Cons: Need parsing logic, potential for invalid formats

**Option B: Separate Fields + Helper (RECOMMENDED)**

```rust
pub llm_model: String,      // "gemma3:12b"
pub llm_provider: String,   // "ollama"

impl Workspace {
    pub fn llm_full_id(&self) -> String {
        format!("{}/{}", self.llm_provider, self.llm_model)
    }
}
```

- Pros: Consistent with embedding fields, easier validation, backward compatible
- Cons: Need to parse combined format in some contexts

**Decision: Option B** - Use separate fields with helper method for combined format.

### Data Model Changes

```rust
// edgequake-core/src/types/multitenancy.rs

pub struct Workspace {
    // ... existing fields ...

    // === LLM Configuration (SPEC-032) ===

    /// LLM model name (e.g., "gemma3:12b", "gpt-4o-mini").
    /// Used for knowledge graph generation, summarization, etc.
    pub llm_model: String,

    /// LLM provider (e.g., "ollama", "openai", "lmstudio").
    /// Determines which API to call for completions.
    pub llm_provider: String,
}

impl Workspace {
    /// Get fully qualified LLM model ID (provider/model format).
    pub fn llm_full_id(&self) -> String {
        format!("{}/{}", self.llm_provider, self.llm_model)
    }

    /// Get fully qualified embedding model ID (provider/model format).
    pub fn embedding_full_id(&self) -> String {
        format!("{}/{}", self.embedding_provider, self.embedding_model)
    }

    /// Parse a full model ID into (provider, model) tuple.
    pub fn parse_model_id(full_id: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = full_id.splitn(2, '/').collect();
        if parts.len() == 2 {
            Some((parts[0].to_string(), parts[1].to_string()))
        } else {
            None
        }
    }
}
```

### API DTO Changes

```rust
// workspaces_types.rs

pub struct CreateWorkspaceApiRequest {
    // ... existing fields ...

    /// LLM model for ingestion/graph generation.
    /// Format: "model_name" or "provider/model_name"
    pub llm_model: Option<String>,

    /// LLM provider (auto-detected if not provided).
    pub llm_provider: Option<String>,
}

pub struct WorkspaceResponse {
    // ... existing fields ...

    pub llm_model: String,
    pub llm_provider: String,
}
```

### Default Value Strategy

```
Priority Order for Workspace LLM Config:
1. Explicit values from CreateWorkspaceApiRequest
2. Server defaults from models.toml [defaults] section
3. Hardcoded fallbacks (openai/gpt-4o-mini)
```

## Implementation Priority

1. **P0: Core Types** - Add fields to Workspace struct
2. **P0: API DTOs** - Add request/response fields
3. **P1: Handlers** - Update create/read/update handlers
4. **P1: Constructor** - Default LLM config from models.toml
5. **P2: WebUI** - Add LLM selector for workspace creation
6. **P3: Documentation** - Update API docs

# OODA Loop Iteration 226 - Orient

## Date: 2026-01-16

## Root Cause Analysis

### Why Does This Duplication Exist?

**Historical Evolution Pattern**:

1. Initially, only non-streaming chat was implemented
2. Streaming was added later by copying the logic
3. Provider resolution grew organically with features
4. No conscious refactoring as complexity increased

**Contributing Factors**:

1. **Tight Coupling**: Error handling is entangled with business logic
2. **Context Dependency**: Streaming needs channel, non-streaming needs Result
3. **Incremental Development**: Each feature added its own provider creation

### Reliability Theory Application

Using the **Failure Mode and Effects Analysis (FMEA)** framework:

| Failure Mode                 | Severity | Occurrence | Detection | RPN |
| ---------------------------- | -------- | ---------- | --------- | --- |
| Safety limit bypass          | 8        | 5          | 3         | 120 |
| Inconsistent error messages  | 4        | 8          | 2         | 64  |
| Provider fallback divergence | 6        | 4          | 4         | 96  |
| API key error hiding         | 7        | 3          | 5         | 105 |

**Risk Priority Number (RPN)** = Severity × Occurrence × Detection

**Top Risks to Address**:

1. Safety limit bypass (RPN 120) - Chat can hang indefinitely
2. API key error hiding (RPN 105) - Users don't know why it fails
3. Provider fallback divergence (RPN 96) - Inconsistent behavior

### First-Principles Design

**Axiom 1**: Provider resolution is a pure function of inputs

- Inputs: workspace_id, request_provider, request_model
- Output: (Provider, provider_name, model_name) or Error

**Axiom 2**: Error handling is a concern of the caller, not the resolver

- The resolver reports what went wrong
- The handler decides how to communicate it

**Axiom 3**: Safety limits are non-negotiable

- Every LLM call MUST have timeout
- Every embedding call MUST have timeout

### Design Pattern: Strategy + Result Monad

```
                    ┌─────────────────────────────────────┐
                    │     WorkspaceProviderResolver        │
                    │                                     │
                    │  ┌─────────────────────────────────┐│
                    │  │   resolve_llm_provider()        ││
                    │  │   - Parses provider/model       ││
                    │  │   - Applies priority logic      ││
                    │  │   - Always uses safe creation   ││
                    │  │   - Returns Result<Provider>    ││
                    │  └─────────────────────────────────┘│
                    │                                     │
                    │  ┌─────────────────────────────────┐│
                    │  │   resolve_embedding_provider()  ││
                    │  │   - Gets workspace config       ││
                    │  │   - Creates safe provider       ││
                    │  │   - Detects API key issues      ││
                    │  │   - Returns Result<Provider>    ││
                    │  └─────────────────────────────────┘│
                    └─────────────────────────────────────┘
                                      │
                                      │ Result<Provider, ResolutionError>
                                      ▼
          ┌───────────────────────────────────────────────────────┐
          │                                                       │
     ┌────┴────┐         ┌─────────┐         ┌─────────────┐     │
     │ chat.rs │         │processor│         │  query.rs   │     │
     │(stream) │         │   .rs   │         │             │     │
     └─────────┘         └─────────┘         └─────────────┘     │
          │                   │                    │              │
          ▼                   ▼                    ▼              │
     ┌─────────┐         ┌─────────┐         ┌─────────┐         │
     │ Send to │         │ Log +   │         │ Return  │         │
     │ Channel │         │ Fallback│         │ ApiError│         │
     └─────────┘         └─────────┘         └─────────┘         │
          │                                                       │
          └───────────────────────────────────────────────────────┘
                         Error Handling Layer
```

### Error Types Design

```rust
/// Provider resolution errors with clear semantics
#[derive(Debug, thiserror::Error)]
pub enum ProviderResolutionError {
    /// Workspace not found in database
    #[error("Workspace not found: {workspace_id}")]
    WorkspaceNotFound { workspace_id: String },

    /// Provider creation failed (API key, network, etc.)
    #[error("Provider creation failed: {provider}/{model}: {reason}")]
    ProviderCreationFailed {
        provider: String,
        model: String,
        reason: String,
        /// True if this is an API key configuration issue
        is_api_key_error: bool,
    },

    /// Invalid workspace ID format
    #[error("Invalid workspace ID format: {0}")]
    InvalidWorkspaceId(String),
}
```

## Strategic Decision

**Recommendation**: Extract provider resolution into a shared module with:

1. Single `WorkspaceProviderResolver` struct
2. Result-based error handling (caller maps to their error type)
3. Always use safe provider creation methods
4. Include API key detection logic universally
5. Consistent logging across all call sites

**Risk Mitigation**:

- All existing tests must pass after refactor
- New tests for extracted logic
- Property-based tests for invariants

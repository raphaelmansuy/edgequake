# OODA-15: Orient

## First Principles Analysis

**Principle**: Production code must never panic on data-dependent paths. A panic in an async web server kills the entire task/thread, potentially cascading to connection drops.

**Approach**: 
1. HIGH risk sites → return `Result` or provide defaults
2. MEDIUM risk sites → clamp NaN/Inf to safe values
3. Safe sites → add `// WHY expect:` comments documenting infallibility
4. RwLock sites → apply poison recovery pattern from OODA-06

**Risk assessment**:
- `reranking.rs` unwrap is guarded by caller — needs verification
- `community.rs` HashMap unwraps are algorithmic invariants — but defensive coding is better
- `pdf_storage_impl.rs` parse unwraps are DB trust assumptions — DB can have invalid data after migration
- `from_f64()` unwraps fail on NaN — floats from LLM parsing can be NaN

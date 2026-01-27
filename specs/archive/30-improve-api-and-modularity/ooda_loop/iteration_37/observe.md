# Iteration 37 - Observe

**Date:** 2026-01-08  
**Focus:** TODO/FIXME audit

## TODO Items Found

| File                             | Line | TODO                                | Priority |
| -------------------------------- | ---- | ----------------------------------- | -------- |
| postgres_conversation_service.rs | 210  | cursor-based pagination             | Medium   |
| postgres_conversation_service.rs | 390  | import functionality                | Low      |
| logger.rs (audit)                | 191  | query execution with dynamic params | Low      |
| orchestrator.rs                  | 901  | Retrieve from KV store              | Medium   |
| orchestrator.rs                  | 1069 | Check all backend connections       | Low      |
| extractor.rs (pdf)               | 313  | Extract images                      | Low      |
| cache.rs (pipeline)              | 358  | Store raw response in cache         | Low      |
| middleware.rs (rate-limiter)     | 81   | Calculate actual reset time         | Medium   |

## Analysis

Most TODOs are for future enhancements, not bugs. Current functionality works.

### High Priority (Would fix now)

None - all are enhancements

### Medium Priority (Should address)

1. **cursor-based pagination** - Currently uses offset (works but less efficient)
2. **KV store retrieval** - Needs implementation for graph stats
3. **rate limiter reset time** - Currently hardcoded to 60

### Low Priority (Nice to have)

1. Import functionality
2. Dynamic params in audit queries
3. Image extraction from PDFs
4. Raw response caching

## Decision

These are documented feature gaps, not code quality issues.
Focus on documenting them properly rather than implementing.

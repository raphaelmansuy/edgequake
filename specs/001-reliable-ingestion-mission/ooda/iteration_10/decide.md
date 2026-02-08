# OODA Iteration 10: Decide

## Date: 2026-02-08

## Decisions

### Decision 1: Verify and Document Optimal Model Configuration

**Priority**: P0 (Critical)
**Signal Value**: High

**Action**:

1. Verify that OpenAI requests actually use `gpt-5-nano` and `text-embedding-3-small`
2. Update documentation comment in model_config.rs (line 31) from `gpt-4o-mini` to `gpt-5-nano`
3. Add high-signal WHY comments about pricing rationale

**Rationale**: Confirm our configuration is optimal before diagnosing user's quota issue.

---

### Decision 2: Test E2E Ingestion with Both Providers

**Priority**: P0 (Critical)
**Signal Value**: High

**Action**:

1. Test document upload with Ollama provider (current default)
2. Restart backend with OpenAI provider environment vars
3. Test document upload with OpenAI provider
4. Verify both produce entity extraction and graph storage

**Rationale**: Mission requires proving both providers work end-to-end.

---

### Decision 3: Update Mission Spec with Model Pricing Findings

**Priority**: P1 (High)
**Signal Value**: Medium

**Action**:

1. Add new success criterion for cost-optimal model configuration
2. Document that quota exceeded is likely user account issue
3. Add troubleshooting guidance for OpenAI quota errors

**Rationale**: Capture learnings for future reference.

---

### Decision 4: Fix Documentation Comment (line 31)

**Priority**: P2 (Medium)
**Signal Value**: Low

**Action**:

1. Update `model_config.rs` line 31 example from `gpt-4o-mini` to `gpt-5-nano`

**Rationale**: Keep documentation accurate, but low priority as it's just a comment.

---

## Implementation Plan

```
┌─────────────────────────────────────────────────────────────────┐
│ Step │ Action                              │ Verification       │
├─────────────────────────────────────────────────────────────────┤
│  1   │ Start backend with Ollama provider  │ curl /health shows │
│      │                                     │ llm_provider=ollama│
├─────────────────────────────────────────────────────────────────┤
│  2   │ Upload test document                │ Document completes │
│      │                                     │ status=completed   │
├─────────────────────────────────────────────────────────────────┤
│  3   │ Restart backend with OpenAI env vars│ curl /health shows │
│      │                                     │ llm_provider=openai│
├─────────────────────────────────────────────────────────────────┤
│  4   │ Upload another test document        │ Document completes │
│      │                                     │ (or captures quota │
│      │                                     │  error clearly)    │
├─────────────────────────────────────────────────────────────────┤
│  5   │ Update model_config.rs comment      │ cargo build passes │
├─────────────────────────────────────────────────────────────────┤
│  6   │ Update mission spec                 │ File updated       │
├─────────────────────────────────────────────────────────────────┤
│  7   │ Run test suite                      │ All tests pass     │
└─────────────────────────────────────────────────────────────────┘
```

## What NOT to Do

1. **DO NOT** change default models - they are already optimal
2. **DO NOT** assume quota issue is code bug - it's likely account limit
3. **DO NOT** over-engineer rate limit handling until we confirm the issue

## Success Metrics for This Iteration

| Metric                        | Target                       |
| ----------------------------- | ---------------------------- |
| Ollama E2E ingestion verified | ✅                           |
| OpenAI E2E ingestion verified | ✅ (or quota error captured) |
| Documentation comment fixed   | ✅                           |
| Mission spec updated          | ✅                           |
| Tests passing                 | 1668+ tests                  |

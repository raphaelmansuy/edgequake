# OODA Iteration 09 - Decide

## Date: 2026-02-08

## Priority 1: Fix Embedding Context Overflow (CRITICAL)

**Decision:** Implement text truncation in Ollama embedding provider

**Rationale:**

- Blocking all ingestion with Ollama
- Simple, defensive fix
- Doesn't require external dependencies

**Implementation Plan:**

1. Find Ollama embedding implementation
2. Add token counting/truncation logic
3. Log warning when truncation occurs
4. Set conservative limit (model_context - 100 tokens buffer)

**Files to modify:**

- `edgequake-llm/src/providers/ollama.rs`

---

## Priority 2: Environment-Based Provider Selection

**Decision:** Make DEFAULT_LLM_PROVIDER and DEFAULT_EMBEDDING_PROVIDER read from environment

**Rationale:**

- Enables battle testing with OpenAI
- Allows CI to run with mock provider
- Production can use appropriate provider

**Environment Variables:**

- `EDGEQUAKE_DEFAULT_LLM_PROVIDER` (default: "ollama")
- `EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER` (default: "ollama")
- `EDGEQUAKE_DEFAULT_LLM_MODEL` (default: "gemma3:12b")
- `EDGEQUAKE_DEFAULT_EMBEDDING_MODEL` (default: "nomic-embed-text")

**Files to modify:**

- `edgequake-core/src/types/multitenancy.rs`
- `Makefile` (export these when OPENAI_API_KEY is set)

---

## Priority 3: Test Ingestion Pipeline

**Decision:** After fixes, test with both providers

**Test Plan:**

1. Upload PDF with Ollama → verify entities created
2. Switch to OpenAI → upload same PDF → verify entities
3. Compare entity quality

---

## Priority 4: Verify Other Success Criteria

**Decision:** Test remaining criteria after ingestion works

**Test Plan:**

1. Delete document → verify PDF storage cleanup
2. Upload 2 documents simultaneously → verify parallel processing
3. Query with both providers → verify responses

---

## Implementation Order

```
+-------------------------------------------+
| Step 1: Truncation Fix (P1)               |
| - Modify ollama.rs                        |
| - Add context-aware truncation            |
| Expected: 30 minutes                      |
+-------------------------------------------+
              │
              ▼
+-------------------------------------------+
| Step 2: Env-Based Providers (P2)          |
| - Modify multitenancy.rs                  |
| - Update Makefile                         |
| Expected: 30 minutes                      |
+-------------------------------------------+
              │
              ▼
+-------------------------------------------+
| Step 3: Test Ollama Ingestion             |
| - Upload test PDF                         |
| - Verify entities created                 |
| Expected: 15 minutes                      |
+-------------------------------------------+
              │
              ▼
+-------------------------------------------+
| Step 4: Test OpenAI Ingestion             |
| - Switch provider                         |
| - Upload same PDF                         |
| - Compare quality                         |
| Expected: 15 minutes                      |
+-------------------------------------------+
              │
              ▼
+-------------------------------------------+
| Step 5: Other Success Criteria            |
| - Delete document test                    |
| - Parallel ingestion test                 |
| - Both providers query test               |
| Expected: 30 minutes                      |
+-------------------------------------------+
```

## Commit Plan

1. `OODA-09a: Add embedding text truncation for Ollama`
2. `OODA-09b: Make default providers environment-configurable`
3. `OODA-09c: Test ingestion with Ollama and OpenAI`

## Go/No-Go

**Go Conditions:**

- ✅ Backend running
- ✅ PostgreSQL healthy
- ✅ Test PDFs available
- ✅ OPENAI_API_KEY set in environment

**Proceed with implementation.**

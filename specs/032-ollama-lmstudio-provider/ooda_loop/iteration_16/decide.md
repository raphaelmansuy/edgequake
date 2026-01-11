# OODA Loop Iteration 16 - Decide Phase

**Date:** 2026-01-11  
**Focus:** Implementation Plan for LLM Provider Override  
**Status:** ✅ COMPLETE

## Decision

Implement the LLM provider override using the same pattern as `query_with_embedding_provider`.

## Implementation Steps

### Step 1: Add `generate_answer_with_provider`

Internal helper that accepts an optional LLM provider override.

### Step 2: Add `query_with_llm_provider`

Public method that:
1. Runs all SOTA pipeline steps (keywords, mode, embeddings, retrieval)
2. Uses the provided LLM for answer generation

### Step 3: Re-export `LLMProvider`

Add to `edgequake-query/src/lib.rs` so consumers can create providers.

### Step 4: Update `chat.rs`

1. Parse `request.provider` ("provider/model" format)
2. Call `ProviderFactory::create_llm_provider`
3. Use `query_with_llm_provider` if override succeeded

## Edge Cases

1. **Empty provider string:** Treat as no override
2. **Provider not found:** Log warning, use default
3. **Model not specified:** Use provider's default

## Test Plan

1. Build and run existing tests
2. Manual test: Select different provider in UI, verify logs

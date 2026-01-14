# Iteration 134 – Observe

## Focus: Model Type Filtering (Item 17)

### Requirement
> **FIX: Model Type Filtering** - Embedding selector must ONLY show embedding models, LLM selector must ONLY show LLM models. The "multimodal" type in EdgeQuake means vision-capable LLM, NOT embedding capability, so multimodal should NOT appear in embedding dropdown.

### Current State

Need to verify:
1. `/api/v1/models/llm` returns only llm + multimodal
2. `/api/v1/models/embedding` returns only embedding
3. UI selectors properly filter models

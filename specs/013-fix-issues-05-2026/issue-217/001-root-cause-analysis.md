# Issue #217 — Root Cause Analysis

**GitHub:** [#217](https://github.com/raphaelmansuy/edgequake/issues/217)

## Symptom (fact)

9 defined entity types → 430 distinct `entity_type` values in production graph.

## 5 WHY

| # | Why | Evidence |
|---|-----|----------|
| 1 | Why 430 types? | LLM emits free-form strings (`TELEPHONE_NUMBER`, `PERSON/ORGANIZATION`, etc.) |
| 2 | Why does LLM invent types? | Prompt: "If none apply, classify as `Other`" — weak constraint |
| 3 | Why no server rejection? | `TupleParser` / `LLMExtractor` accepted any `entity_type` string |
| 4 | Why workspace types ignored? | Types passed to prompt but not enforced post-parse |
| 5 | Why NULL types? | Parser skips malformed tuples; empty type field |

## Fix summary

1. Stricter prompt (ONLY listed types)  
2. `enforce_entity_type()` in pipeline post-parse (LLM + SOTA paths)  
3. Unit tests in `entity_type_policy.rs`

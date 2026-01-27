# OODA-30 Decide: Add Performance Baseline Test

## Test Design

### test_deletion_performance_baseline

1. Upload document with measurable content
2. Time the deletion operation
3. Assert reasonable performance (<100ms for in-memory)
4. Print timing for benchmarking

### Key Metrics

- Total deletion time (ms)
- Entities affected
- Relationships affected
- Embeddings removed

## Note

With mock provider, entity extraction is minimal. This test establishes
the framework for future performance testing with real LLM.

## Implementation

Add to e2e_document_deletion.rs

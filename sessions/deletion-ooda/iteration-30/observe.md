# OODA-30 Observe: Performance Benchmarking

## Gap Identified

We have functional tests but no performance benchmarks. Need to measure:

1. Deletion time vs entity count
2. Deletion time vs relationship count
3. Memory usage during deletion
4. Cascade overhead

## Current State

- 41 tests covering correctness
- No performance tests
- Study goal: <100ms deletion for 100K nodes

## Approach

Use simple timing tests to establish baseline:
1. Small document (10 entities)
2. Medium document (100 entities)
3. Document with many relationships

## Implementation

Add `test_deletion_performance_baseline` to e2e_document_deletion.rs

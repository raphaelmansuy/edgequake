# Observe: Multi-Document Knowledge Graph Test

## Objective

Test KG with multiple documents to verify:

1. Entity deduplication across documents
2. Relationship merging
3. Query aggregation from multiple sources

## Current State

- 1 document uploaded (knowledge-graphs.txt)
- 12 entities extracted
- Query works correctly

## Test Plan

1. Upload a second document with overlapping entities
2. Verify entity deduplication
3. Test cross-document queries
4. Verify rebuild clears all documents

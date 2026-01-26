# OODA-28 Observe: Deletion Edge Case Gap Analysis

## Current Test Coverage (30 tests)

Looking at e2e_document_deletion.rs, tests cover:
- Basic document deletion
- Entity/relationship cascade
- Embedding cleanup
- Multiple document handling
- Metadata scenarios
- Concurrent operations
- No-entity documents
- Rapid sequential ops
- Isolation verification

## Potential Gaps

### 1. Large Document Tests
- Document with many entities (100+)
- Deep relationship chains
- High embedding count

### 2. Unicode/Special Content
- Documents with unicode names
- Special characters in content
- Empty document name edge case

### 3. Timing Edge Cases
- Delete during processing (if possible)
- Delete same document twice rapidly
- Delete then immediately re-upload

### 4. Storage-Specific
- PostgreSQL with real database
- Memory storage limits

## Priority: Add 3 more edge case tests

Focus on:
1. Unicode document names
2. Double-delete (idempotency)
3. Delete-then-reupload

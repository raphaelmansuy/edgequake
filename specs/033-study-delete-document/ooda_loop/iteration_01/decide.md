# ITERATION 01 - DECIDE

**Mission**: Study document add/delete process on EdgeQuake

**Date**: 2026-01-26

**Previous Phase**: [ORIENT](./orient.md) - Analyzed gaps and designed solutions

---

## Decision Framework

**Criteria for ITERATION 01**:
1. **Immediate Impact**: Fix critical bugs that cause data loss
2. **Foundation for Future**: Changes that enable subsequent optimizations
3. **Low Risk**: Minimize risk of breaking existing functionality
4. **Testable**: Can be validated with automated tests

**Out of Scope for ITERATION 01**:
- Complex architectural changes (saga pattern)
- New features (soft delete, batch API)
- Long-term infrastructure (orphan cleanup service)

---

## Selected Changes for ITERATION 01

### ✅ CHANGE-01: Fix Edge Deletion Race Condition (GAP-03)

**Priority**: P0 - CRITICAL BUG

**Problem**:
When deleting a document, all edges connected to an entity are deleted, even if those edges have other source documents. This causes data loss.

**Example**:
```
Document A: "Alice works at Google"
Document B: "Alice graduated from MIT"

DELETE Document A:
  Result: Both "ALICE → GOOGLE" and "ALICE → MIT" edges are deleted
  Expected: Only "ALICE → GOOGLE" edge should be deleted
```

**Solution**:
Before deleting an edge, check if the edge itself references the document being deleted.

**Implementation Plan**:

1. **Update `delete_document` function** (documents.rs ~line 1474)
   - Add edge source checking before deletion
   - Only delete edges that have no remaining sources

2. **Add helper function** `should_delete_edge()`
   - Extract edge source checking logic
   - Make it reusable and testable

3. **Add integration test**
   - Create scenario with two documents sharing an entity
   - Delete one document, verify other document's edges remain

**Files to Modify**:
- `edgequake/crates/edgequake-api/src/handlers/documents.rs`

**Acceptance Criteria**:
- ✅ Edge is deleted ONLY if its `source_ids` array becomes empty
- ✅ Edge is preserved if it has other source documents
- ✅ Integration test passes for multi-document entity scenario
- ✅ All existing tests still pass

**Estimated Effort**: 2-3 hours
**Risk**: Low (surgical fix, well-contained)

---

### ✅ CHANGE-02: Add Query-by-Property Methods to GraphStorage Trait (GAP-02)

**Priority**: P1 - HIGH PERFORMANCE

**Problem**:
Current deletion scans the entire graph (`get_all_nodes()`, `get_all_edges()`) to find entities/relationships that reference a document. This is O(N) and becomes prohibitively slow for large graphs.

**Solution**:
Add indexed query methods to `GraphStorage` trait to find nodes/edges by property values.

**Implementation Plan**:

1. **Extend GraphStorage trait** (traits/graph.rs)
   - Add `get_nodes_by_array_contains(property_key, search_value)`
   - Add `get_edges_by_array_contains(property_key, search_value)`
   - Add documentation with performance expectations

2. **Implement for MemoryGraphStorage** (adapters/memory/graph.rs)
   - Linear scan implementation (O(N), acceptable for tests)
   - No index needed for memory storage

3. **Implement for PostgresAGEStorage** (adapters/postgres_age/graph.rs)
   - Use JSON containment operator: `properties @> '{"source_ids": ["doc-123"]}'`
   - Create GIN index on `properties` column if not exists
   - Query: `SELECT * FROM nodes WHERE properties @> ...`

4. **Update `delete_document` to use new API** (handlers/documents.rs)
   - Replace `get_all_nodes()` with `get_nodes_by_array_contains()`
   - Replace `get_all_edges()` with `get_edges_by_array_contains()`
   - Remove full graph iteration

5. **Add performance benchmark**
   - Create test with 10K, 100K nodes
   - Measure deletion time before/after
   - Verify 10x-100x improvement

**Files to Modify**:
- `edgequake/crates/edgequake-storage/src/traits/graph.rs`
- `edgequake/crates/edgequake-storage/src/adapters/memory/graph.rs`
- `edgequake/crates/edgequake-storage/src/adapters/postgres_age/graph.rs` (if exists)
- `edgequake/crates/edgequake-api/src/handlers/documents.rs`

**Acceptance Criteria**:
- ✅ Trait methods added with clear documentation
- ✅ Memory implementation works (tests pass)
- ✅ PostgreSQL implementation uses indexed queries
- ✅ Deletion performance improves by 10x for 100K nodes
- ✅ All existing tests pass

**Estimated Effort**: 1-2 days
**Risk**: Medium (changes storage abstraction, needs careful testing)

---

### ✅ CHANGE-03: Add Comprehensive Integration Test Suite

**Priority**: P1 - FOUNDATION

**Problem**:
No integration tests for document deletion scenarios, especially cascade behavior with shared entities.

**Solution**:
Create comprehensive test suite covering all deletion edge cases.

**Test Scenarios**:

1. **Test: Single document deletion**
   - Add document, verify entities/edges created
   - Delete document, verify all data removed
   - Verify embeddings cleaned up

2. **Test: Multi-document shared entity**
   - Add Document A: "Alice works at Google"
   - Add Document B: "Alice graduated from MIT"
   - Verify ALICE entity has sources: [doc_a, doc_b]
   - Delete Document A
   - Verify ALICE entity still exists with sources: [doc_b]
   - Verify GOOGLE entity deleted
   - Verify MIT entity and edge still exist

3. **Test: Workspace isolation during deletion**
   - Add documents to workspace_1 and workspace_2
   - Delete document from workspace_1
   - Verify workspace_2 data unaffected
   - Verify vector storage isolation maintained

4. **Test: Deletion impact analysis**
   - Add complex document with many entities
   - Call `/api/v1/documents/{id}/deletion-impact`
   - Verify metrics are accurate (entities to remove/update)
   - Perform actual deletion
   - Verify impact analysis was correct

5. **Test: Failed deletion recovery** (if saga implemented)
   - Simulate failure at various stages
   - Verify rollback/compensation works
   - Verify no partial deletion artifacts

**Files to Create**:
- `edgequake/crates/edgequake-api/tests/integration/document_deletion_test.rs`

**Acceptance Criteria**:
- ✅ All 4 test scenarios pass
- ✅ Tests use both Memory and PostgreSQL backends (via feature flags)
- ✅ Tests verify all storage layers (KV, Graph, Vector)
- ✅ Tests include assertions on metrics (entities_removed, relationships_removed, etc.)

**Estimated Effort**: 1 day
**Risk**: Low (new tests, no risk to existing code)

---

### ✅ CHANGE-04: Add Documentation and WHY Comments

**Priority**: P2 - DOCUMENTATION

**Problem**:
Complex deletion logic is not well-documented. Future maintainers will struggle to understand cascade behavior.

**Solution**:
Add comprehensive comments explaining WHY decisions were made.

**Documentation Updates**:

1. **documents.rs - delete_document function**
   - Add ASCII diagram of deletion flow
   - Explain cascade logic
   - Document why edges are checked before deletion
   - Explain source_ids tracking mechanism

2. **traits/graph.rs**
   - Document performance expectations for new query methods
   - Explain when to use `get_nodes_by_array_contains` vs `get_all_nodes`

3. **Create deletion design document** (specs/033-study-delete-document/docs/deletion-design.md)
   - High-level architecture
   - Cascade logic explanation
   - Performance characteristics
   - Future optimization opportunities

**Files to Modify/Create**:
- `edgequake/crates/edgequake-api/src/handlers/documents.rs`
- `edgequake/crates/edgequake-storage/src/traits/graph.rs`
- `specs/033-study-delete-document/docs/deletion-design.md` (new)

**Acceptance Criteria**:
- ✅ All key decision points have WHY comments
- ✅ ASCII diagrams added for complex flows
- ✅ Design document provides 10,000-foot view
- ✅ Links to FEAT/BR/UC IDs where applicable

**Estimated Effort**: 4 hours
**Risk**: None (documentation only)

---

## Deferred to Future Iterations

### ⏸️ DEFERRED: Saga Pattern for Atomic Deletion (GAP-01)

**Reason**: 
- High complexity, requires careful design
- Needs broader discussion (affects all mutation operations, not just deletion)
- Current fix (CHANGE-01) addresses immediate data integrity concern
- Performance improvement (CHANGE-02) is more impactful in short term

**Recommendation**: 
- Address in ITERATION 02 after validating CHANGE-01 and CHANGE-02
- Consider if really needed after fixing race condition bug

---

### ⏸️ DEFERRED: Batch Delete API (GAP-06)

**Reason**:
- Depends on CHANGE-02 (query-by-property) for efficient implementation
- Not critical for MVP (users can delete one by one)
- Better to validate single-document deletion first

**Recommendation**:
- Implement in ITERATION 03 after CHANGE-02 is proven in production
- Design API during ITERATION 02 (planning phase)

---

### ⏸️ DEFERRED: Soft Delete (GAP-05)

**Reason**:
- Feature request, not a bug fix
- Requires schema changes and migration
- Needs product discussion (do users really need this?)

**Recommendation**:
- Gather user feedback first
- If validated, implement in ITERATION 04+
- Not critical for initial release

---

### ⏸️ DEFERRED: Orphan Cleanup Service (GAP-04)

**Reason**:
- Technical debt mitigation, not immediate issue
- Current fix (CHANGE-01) prevents new orphans from being created
- Can run manual cleanup script if needed

**Recommendation**:
- Monitor production for orphan growth after CHANGE-01 deployed
- If orphans accumulate, implement in ITERATION 03
- Otherwise, not urgent

---

## Implementation Order

### Week 1, Day 1-2: Critical Bug Fix
1. CHANGE-01: Fix edge deletion race condition
   - Implement fix
   - Add focused integration test
   - Code review and merge

### Week 1, Day 3-5: Performance Optimization
2. CHANGE-02: Add query-by-property API
   - Extend trait
   - Implement for Memory storage
   - Implement for PostgreSQL AGE storage
   - Update delete_document to use new API
   - Benchmark performance

### Week 2, Day 1-2: Testing & Validation
3. CHANGE-03: Integration test suite
   - Implement all 4 test scenarios
   - Run against both backends
   - Fix any issues found

### Week 2, Day 3: Documentation
4. CHANGE-04: Documentation and WHY comments
   - Add inline comments
   - Create design document
   - Update FEAT/BR/UC cross-references

---

## Success Metrics

### Pre-Implementation Baseline (Measure Now)
```bash
# Create graph with 10K nodes
# Measure deletion time
# Count lines of test coverage for deletion
# Record user-reported "missing data" issues
```

### Post-Implementation Targets
- ✅ Zero data loss bugs (CHANGE-01 validates this)
- ✅ Deletion time < 500ms for 10K nodes, < 5s for 100K nodes
- ✅ 90%+ test coverage for deletion logic
- ✅ All existing tests pass (no regressions)

---

## Risk Mitigation Plan

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| PostgreSQL AGE implementation breaks existing queries | Low | High | Thorough testing, rollback plan |
| Performance improvement not as expected | Medium | Medium | Benchmark before/after, optimize if needed |
| New tests reveal additional bugs | High | Low | Good! Fix them before release |
| Backward compatibility break | Low | High | Version API, use feature flags |

---

## Rollback Strategy

If CHANGE-02 causes issues in production:
1. Feature flag to disable new query methods
2. Fallback to `get_all_nodes()` temporarily
3. Investigate and fix issue
4. Re-enable optimized path

---

## Next Steps (ACT Phase)

1. Create feature branch: `feat/delete-document-improvements`
2. Implement CHANGE-01 (edge deletion fix)
3. Write integration test for CHANGE-01
4. Submit PR for review
5. After merge, begin CHANGE-02 (performance optimization)
6. Continue through implementation order

**Ready to ACT**: All decisions documented, plan is clear, risks identified.

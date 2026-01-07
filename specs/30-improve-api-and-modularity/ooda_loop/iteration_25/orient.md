# Iteration 25: Orient

## Analysis

### Root Cause
The [documents.rs](../../../edgequake/crates/edgequake-api/src/handlers/documents.rs) file violates the Single Responsibility Principle by combining:
- Multiple distinct operations (upload, list, get, delete, batch)
- DTOs, validation, and business logic
- Helper functions for cost, deduplication, and processing

### Architectural Pattern
Currently: **Monolithic Handler Pattern**
```
documents.rs (3,573 lines)
├── upload_document()
├── list_documents()
├── get_document()
├── delete_document()
├── analyze_deletion_impact()
├── upload_file()
├── batch_upload()
└── 15+ DTOs + helpers
```

Proposed: **Modular Handler Pattern**
```
documents/
├── mod.rs          (re-exports)
├── dtos.rs         (all DTOs)
├── upload.rs       (upload_document + helpers)
├── list.rs         (list_documents + helpers)
├── detail.rs       (get_document + helpers)
├── delete.rs       (delete + impact analysis)
├── files.rs        (file upload handlers)
└── batch.rs        (batch operations)
```

### Benefits of Extraction

1. **Single Responsibility**
   - Each module handles one domain concern
   - Easier to reason about code behavior
   
2. **Reduced Cognitive Load**
   - ~400-600 lines per file vs 3,573
   - Find functionality faster

3. **Parallel Development**
   - Team can work on upload without touching delete logic
   - Reduced merge conflicts

4. **Testability**
   - Tests co-located with implementations
   - Isolated test failures point to specific module

5. **Maintenance**
   - Changes to one operation don't risk others
   - Clear boundaries for refactoring

### Risks & Mitigation

| Risk                          | Mitigation                                   |
|-------------------------------|----------------------------------------------|
| Breaking imports              | Keep public API in mod.rs                   |
| Test failures                 | Run full suite after each extraction        |
| Lost cohesion                 | Group by operation, not by artifact type    |
| Over-fragmentation            | Limit to 7 modules (Miller's Law)           |

### Extraction Strategy

**Phase 1: Create structure (non-breaking)**
1. Create `handlers/documents/` directory
2. Create `mod.rs` with re-exports
3. Move DTOs to `dtos.rs`
4. Update imports, run tests

**Phase 2: Extract handlers (one at a time)**
5. Move `upload_document` + helpers → `upload.rs`
6. Test, commit
7. Repeat for list, detail, delete, files, batch

**Phase 3: Cleanup**
8. Remove original `documents.rs`
9. Update documentation
10. Final test run

### Decision

✅ Proceed with modular extraction
- Target: 7 focused modules
- Strategy: Incremental, test-driven
- Timeline: 1 iteration (multiple commits)

Next: Decide phase for implementation plan.

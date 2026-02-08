# OODA Iteration 02 - Decide

## Mission Re-Read Checkpoint
✅ Mission file re-read: `./specs/001-reliable-ingestion-mission.md`

## Decision: Focused Safety Hardening

Based on the orientation analysis, this iteration will focus on **safety hardening** - ensuring production configurations are safe by default.

### Priority 1: Verify Current Test Status

**Decision:** Quick test run to check baseline before changes.

**Acceptance Criteria:**
- [ ] `cargo test -p edgequake-pipeline` completes
- [ ] Document any failures for fixing

### Priority 2: Deprecate gpt-4o-mini Constructor

**Decision:** Mark the `new_gpt4o_mini()` function as deprecated, encourage gpt-5-nano.

**Files to Modify:**
- `edgequake/crates/edgequake-pipeline/src/progress.rs`
  - Line ~610: Add `#[deprecated]` attribute to `new_gpt4o_mini()`
  - Update doc comment to recommend `new_gpt5_nano()`

**NOT Changing:**
- HashMap pricing entries (valid for tracking legacy costs)
- Test assertions that verify map contains models

### Priority 3: Enhance Memory Mode Warning

**Decision:** Make the in-memory warning more explicit about production use.

**Files to Modify:**
- `edgequake/src/main.rs`
  - Line ~254: Update warning message

**New Message:**
```
⚠️ WARNING: No DATABASE_URL set - using IN-MEMORY storage.
   Data WILL NOT PERSIST across restarts. NOT FOR PRODUCTION USE.
   Set DATABASE_URL to use PostgreSQL for production.
```

### Priority 4: Add Makefile Safety Documentation

**Decision:** Add WARNING comments to `backend-memory` target.

**Files to Modify:**
- `Makefile`
  - `backend-memory` target: Add explicit NOT FOR PRODUCTION warning

### Not In Scope (Deferred)

- Creating new tests for Makefile DATABASE_URL validation (iteration 03+)
- Full test suite verification (waiting for clean baseline)
- Comprehensive documentation updates (iteration 03+)

## Implementation Checklist

1. [ ] Run quick test to verify current state
2. [ ] Add `#[deprecated]` to `new_gpt4o_mini()` in progress.rs
3. [ ] Update memory mode warning in main.rs
4. [ ] Add WARNING to Makefile `backend-memory` target
5. [ ] Run tests to verify no regressions
6. [ ] Commit with message `OODA-02: Safety hardening for production mode`

## Rationale

**First Principles:**
- Small, focused changes are safer than large refactors
- Deprecation warnings guide users without breaking existing code
- Explicit warnings prevent silent failures
- Each iteration should produce verifiable, committable changes

## Risk Assessment

| Change | Risk | Mitigation |
|--------|------|------------|
| Deprecate function | Low | Function still works, just warns |
| Update warning msg | None | Informational only |
| Makefile comment | None | Documentation only |

## Success Criteria for This Iteration

- [x] Observe.md created
- [x] Orient.md created
- [x] Decide.md created
- [ ] Act.md created with:
  - [ ] Specific line changes documented
  - [ ] Commit SHA recorded
  - [ ] Test results documented

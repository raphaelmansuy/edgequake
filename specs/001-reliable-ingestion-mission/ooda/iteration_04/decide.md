# OODA Iteration 04 - Decide

## Mission Re-Read Checkpoint
✅ Mission file re-read: `./specs/001-reliable-ingestion-mission.md`

## Decision: Test Fix + Code Cleanup

This iteration focuses on:
1. Verifying the test fix from observe phase
2. Fixing unused code warnings in production code
3. Quick E2E verification

## Specific Changes

### Change 1: Test Fix (Already Applied)

**File:** `edgequake-api/src/handlers/pdf_upload.rs:1520`
**Status:** ✅ Done in observe phase

```rust
// OODA-04: Updated from gpt-4o-mini to gpt-5-nano per mission directive
assert_eq!(opts.vision_model(), "gpt-5-nano");
```

### Change 2: Remove Unused Import

**File:** `edgequake-api/src/handlers/documents.rs:55`

**Current:**
```rust
use edgequake_storage::ListPdfFilter;
```

**Action:** Remove this line

### Change 3: Remove Unnecessary Mutable

**File:** `edgequake-api/src/handlers/documents.rs:2527`

**Current:**
```rust
let mut total_pdfs_deleted = 0usize;
```

**New:**
```rust
let total_pdfs_deleted = 0usize;
```

### Change 4: Quick E2E Check

**Steps:**
1. Check test documents exist in `zz-explore/EMILE_FREY/`
2. Restart backend with DATABASE_URL
3. Upload PDF via API/curl

**Not In Scope (This Iteration):**
- Playwright browser automation
- Full dead code cleanup in test files
- Comprehensive E2E testing

## Implementation Checklist

1. [x] Apply test fix (done in observe)
2. [ ] Remove unused import in documents.rs
3. [ ] Remove unnecessary `mut` in documents.rs
4. [ ] Verify test suite passes
5. [ ] Verify test documents exist
6. [ ] Commit with `OODA-04: Fix test + cleanup unused code`

## Risk Assessment

| Change | Risk | Impact |
|--------|------|--------|
| Remove unused import | None | Cleaner code |
| Remove `mut` | None | Cleaner code |
| Test fix | Low | May need adjustment |

## Success Criteria

- [ ] `cargo test -p edgequake-api --lib` passes all tests
- [ ] No warning for `ListPdfFilter` unused import
- [ ] No warning for unnecessary mutable
- [ ] Test documents directory confirmed to exist

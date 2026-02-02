# OODA-83: Decide

**Date**: 2026-02-01
**Mission Re-read**: ✅

## Decision: Create ContentHasher Service

### Implementation Steps

1. **Create services module**
   - `edgequake-api/src/services/mod.rs`
   - `edgequake-api/src/services/content_hasher.rs`

2. **Register module in lib.rs**
   - Add `pub mod services;`

3. **Update documents.rs**
   - Replace 3 hash computation locations with ContentHasher calls
   - Remove duplicate imports (sha2::Sha256)

### Files to Create/Modify

| Action | File                                           |
| ------ | ---------------------------------------------- |
| CREATE | `edgequake-api/src/services/mod.rs`            |
| CREATE | `edgequake-api/src/services/content_hasher.rs` |
| MODIFY | `edgequake-api/src/lib.rs`                     |
| MODIFY | `edgequake-api/src/handlers/documents.rs`      |

---

## Acceptance Criteria

- [ ] ContentHasher service created with hash_bytes, hash_str, workspace_hash_key
- [ ] All 3 hash computation locations use ContentHasher
- [ ] Consistent hex output format
- [ ] cargo build succeeds
- [ ] cargo test passes

---

## Next Action

Proceed to **Act** phase to implement the changes.

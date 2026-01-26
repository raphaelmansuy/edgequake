# OODA-28 Decide: Add Unicode and Idempotency Tests

## Tests to Add

### 1. test_delete_document_unicode_name
Upload document with unicode characters in name, verify deletion works.

### 2. test_delete_document_double_delete
Delete same document twice, verify second returns 404 (idempotent).

### 3. test_delete_then_reupload_same_name
Delete document, re-upload with same name, verify fresh state.

## Implementation Location
Add to: `edgequake/crates/edgequake-api/tests/e2e_document_deletion.rs`

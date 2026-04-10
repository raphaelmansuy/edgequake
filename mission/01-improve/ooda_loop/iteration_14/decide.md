# OODA-14: Decide

1. Add `ResultExt` import to all 12 remaining handler files
2. Mechanically replace all 49 `map_err(|e| ApiError::Internal(format!(...)))` sites with `internal_err("context")`
3. Replace 2 UUID parse sites with `parse_uuid()` in session.rs
4. Replace fully-qualified `crate::error::ApiError` in document_filter_resolver.rs with imported `ApiError`
5. Fix unused import warnings from prior iterations (stuck.rs, reprocess.rs, operations.rs)
6. Run `cargo test -p edgequake-api --lib` and `cargo clippy`

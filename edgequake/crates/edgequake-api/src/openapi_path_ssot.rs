//! Compile-time OpenAPI path registry SSOT (SPEC-027 phase 15).
//!
//! `build.rs` scans handler annotations and validates they match `openapi.rs` paths().

include!(concat!(env!("OUT_DIR"), "/openapi_path_count.rs"));

/// Handler function names registered in `openapi.rs` `paths()` (last path segment).
///
/// EN-3677 Phase 1 bumped this from 200 → 202: `copy_workspace` and
/// `get_workspace_copy_job`. `delete_workspace` was already registered
/// (added by an earlier commit) so it is not counted twice.
pub const REGISTERED_HANDLER_COUNT: usize = 202;

const _: () = assert!(
    OPENAPI_GENERATED_HANDLER_COUNT == REGISTERED_HANDLER_COUNT,
    "openapi.rs paths() count must match build.rs scan — run build and update REGISTERED_HANDLER_COUNT"
);

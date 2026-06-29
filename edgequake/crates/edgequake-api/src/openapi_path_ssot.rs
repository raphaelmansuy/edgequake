//! Compile-time OpenAPI path registry SSOT (SPEC-027 phase 15).
//!
//! `build.rs` scans handler annotations and validates they match `openapi.rs` paths().

include!(concat!(env!("OUT_DIR"), "/openapi_path_count.rs"));

/// Handler function names registered in `openapi.rs` `paths()` (last path segment).
pub const REGISTERED_HANDLER_COUNT: usize = 169;

const _: () = assert!(
    OPENAPI_GENERATED_HANDLER_COUNT == REGISTERED_HANDLER_COUNT,
    "openapi.rs paths() count must match build.rs scan — run build and update REGISTERED_HANDLER_COUNT"
);

//! Services module for shared business logic.
//!
//! WHY-OODA83: Extracted services follow SRP and DRY principles.
//! Consolidates repeated logic into single, testable modules.

pub mod audit;
pub mod content_hasher;
pub mod query_execution;

pub use audit::{record_audit, record_compliance_event, with_request_context};

pub use content_hasher::ContentHasher;
pub use query_execution::{
    execute_sota_query, execute_sota_query_stream, execute_sota_query_stream_with_auth_fallback,
    execute_sota_query_with_auth_fallback, is_llm_auth_failure, llm_override_from_request,
    resolve_workspace_query_resources, validate_llm_override_pair, WorkspaceQueryResources,
};

//! Services module for shared business logic.
//!
//! WHY-OODA83: Extracted services follow SRP and DRY principles.
//! Consolidates repeated logic into single, testable modules.

pub mod content_hasher;
pub mod query_execution;

pub use content_hasher::ContentHasher;
pub use query_execution::{
    execute_sota_query, execute_sota_query_stream, llm_override_from_request,
    resolve_workspace_query_resources, WorkspaceQueryResources,
};

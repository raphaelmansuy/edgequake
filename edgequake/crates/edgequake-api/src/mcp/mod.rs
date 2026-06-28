//! EdgeQuake MCP server module (SPEC-028 SOTA).
//!
//! Streamable HTTP gateway, OAuth resource metadata, and tool dispatch SSOT.

pub mod auth;
pub mod config;
pub mod gateway;
pub mod registry;

pub use auth::protected_resource_metadata;
pub use config::McpPublicConfig;
pub use gateway::handle_mcp_request;
pub use registry::{build_registry_manifest, REGISTRY_SERVER_NAME, SERVER_JSON_SCHEMA};

//! EQ-MCP-1.0 AgentView projection (SPEC-152).
//!
//! Maps REST/query DTOs into the agent envelope. Transport stays in `gateway/`.

pub mod assets;
pub mod blob;
pub mod budget;
pub mod catalog;
pub mod cursor;
pub mod doc_scope;
pub mod doc_text;
pub mod download;
pub mod edge_filter;
pub mod entity_ref;
pub mod envelope;
pub mod errors;
pub mod fetch;
pub mod graph;
pub mod graph_image;
pub mod graph_layout;
pub mod graph_raster;
pub mod graph_theme;
pub mod ids;
pub mod ingest;
pub mod profile;
pub mod profile_gate;
pub mod scores;
pub mod search;
pub mod summary;
pub mod upload_session;

pub use budget::{apply_budget, BudgetClass};
pub use envelope::{truncation_ok, EnvelopeBuilder};
pub use errors::{eq_error, ErrorCode};
pub use profile::{mcp_profile, McpProfile, MEMORY_INSTRUCTIONS, QUERY_INSTRUCTIONS};
pub use summary::{call_tool_error_structured, call_tool_result, call_tool_result_with_image};

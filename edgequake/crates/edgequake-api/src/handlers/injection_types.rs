//! DTOs for knowledge injection handlers.
//!
//! @implements SPEC-0002 (Knowledge Injection)

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Maximum injection content size: 100KB.
pub const MAX_INJECTION_CONTENT_BYTES: usize = 100 * 1024;

// ============================================================================
// Request Types
// ============================================================================

/// Request body for creating/updating a knowledge injection (text mode).
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PutInjectionRequest {
    /// Human-readable name for this injection entry.
    pub name: String,
    /// Plain-text injection content (glossary, acronyms, definitions).
    pub content: String,
}

/// Request body for updating an existing injection entry.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateInjectionRequest {
    /// New name (optional — keeps existing if omitted).
    pub name: Option<String>,
    /// New content (optional — if provided, re-triggers pipeline processing).
    pub content: Option<String>,
}

// ============================================================================
// Response Types
// ============================================================================

/// Response for PUT injection (accepted for processing).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PutInjectionResponse {
    /// Unique injection entry ID.
    pub injection_id: String,
    /// Workspace this injection belongs to.
    pub workspace_id: String,
    /// Version number (increments on each replace).
    pub version: u32,
    /// Processing status.
    pub status: String,
}

/// Response for GET injection (single entry).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct InjectionDetailResponse {
    /// Unique injection entry ID.
    pub injection_id: String,
    /// Human-readable name.
    pub name: String,
    /// Original injection content.
    pub content: String,
    /// Version number.
    pub version: u32,
    /// Processing status.
    pub status: String,
    /// Number of entities extracted.
    pub entity_count: u32,
    /// Source type: "text" or "file".
    pub source_type: String,
    /// Error message if processing failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// When created (ISO 8601).
    pub created_at: String,
    /// When last updated (ISO 8601).
    pub updated_at: String,
}

/// Response for listing injection entries.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListInjectionsResponse {
    /// List of injection entries.
    pub items: Vec<InjectionSummary>,
    /// Total count in workspace.
    pub total: usize,
    /// Page limit applied.
    pub limit: usize,
    /// Page offset applied.
    pub offset: usize,
    /// True when more items exist after this page.
    pub has_more: bool,
}

/// Query parameters for paginated injection list (SPEC-025 6.5).
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ListInjectionsQuery {
    #[serde(default = "default_injection_list_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

fn default_injection_list_limit() -> usize {
    crate::services::DEFAULT_INJECTION_LIST_LIMIT
}

/// Summary of a single injection entry (for list view).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct InjectionSummary {
    /// Unique injection entry ID.
    pub injection_id: String,
    /// Human-readable name.
    pub name: String,
    /// Processing status.
    pub status: String,
    /// Number of entities extracted.
    pub entity_count: u32,
    /// Source type: "text" or "file".
    pub source_type: String,
    /// Error message if processing failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// When created (ISO 8601).
    pub created_at: String,
    /// When last updated (ISO 8601).
    pub updated_at: String,
}

/// Response for DELETE injection.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DeleteInjectionResponse {
    /// Whether the injection was deleted.
    pub deleted: bool,
    /// Message.
    pub message: String,
}

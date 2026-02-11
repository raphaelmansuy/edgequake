//! Common shared types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Health check response.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HealthResponse {
    pub status: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub storage_mode: Option<String>,
    #[serde(default)]
    pub components: Option<HashMap<String, bool>>,
}

/// Pagination info returned in list responses.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PaginationInfo {
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub per_page: Option<u32>,
    #[serde(default)]
    pub total: Option<u64>,
    #[serde(default)]
    pub total_pages: Option<u32>,
}

/// Status counts for documents.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StatusCounts {
    #[serde(default)]
    pub uploading: Option<u32>,
    #[serde(default)]
    pub processing: Option<u32>,
    #[serde(default)]
    pub completed: Option<u32>,
    #[serde(default)]
    pub failed: Option<u32>,
}

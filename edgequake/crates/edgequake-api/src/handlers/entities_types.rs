//! Entity DTO types.
//!
//! This module contains all Data Transfer Objects for the entity management API.
//! Extracted from entities.rs for modularity and single responsibility.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ============================================================================
// Default value helper functions
// ============================================================================

/// Default merge strategy.
pub fn default_merge_strategy() -> String {
    "prefer_target".to_string()
}

/// Default true for delete_relationships (entities module).
pub fn entities_default_true() -> bool {
    true
}

// ============================================================================
// Request DTOs
// ============================================================================

/// Create entity request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateEntityRequest {
    /// Entity name (will be normalized to UPPERCASE).
    pub entity_name: String,

    /// Entity type (e.g., PERSON, ORGANIZATION, TECHNOLOGY).
    pub entity_type: String,

    /// Entity description.
    pub description: String,

    /// Source document ID (use "manual_entry" for manual entries).
    pub source_id: String,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Update entity request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateEntityRequest {
    /// Updated entity type.
    pub entity_type: Option<String>,

    /// Updated description.
    pub description: Option<String>,

    /// Updated metadata.
    pub metadata: Option<serde_json::Value>,
}

/// Merge entities request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct MergeEntitiesRequest {
    /// Source entity to merge from.
    pub source_entity: String,

    /// Target entity to merge into.
    pub target_entity: String,

    /// Merge strategy: "prefer_source", "prefer_target", "merge".
    #[serde(default = "default_merge_strategy")]
    pub merge_strategy: String,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Delete query parameters.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct DeleteEntityQuery {
    /// Whether to delete relationships (default: true).
    #[serde(default = "entities_default_true")]
    pub delete_relationships: bool,

    /// Confirmation flag (required).
    pub confirm: bool,
}

/// Entity exists query.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct EntityExistsQuery {
    /// Entity name to check.
    pub entity_name: String,
}

/// List entities query parameters.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ListEntitiesQuery {
    /// Page number (1-indexed).
    #[serde(default = "default_page")]
    pub page: u32,

    /// Page size (default 20, max 100).
    #[serde(default = "default_page_size")]
    pub page_size: u32,

    /// Filter by entity type.
    pub entity_type: Option<String>,

    /// Search term for entity name or description.
    pub search: Option<String>,
}

/// Default page number.
fn default_page() -> u32 {
    1
}

/// Default page size.
fn default_page_size() -> u32 {
    20
}

// ============================================================================
// Response DTOs
// ============================================================================

/// Paginated list of entities response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListEntitiesResponse {
    /// List of entities.
    pub items: Vec<EntityResponse>,

    /// Total number of entities matching the query.
    pub total: usize,

    /// Current page number.
    pub page: u32,

    /// Page size.
    pub page_size: u32,

    /// Total number of pages.
    pub total_pages: u32,
}

/// Entity response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EntityResponse {
    /// Entity ID.
    pub id: String,

    /// Entity name.
    pub entity_name: String,

    /// Entity type.
    pub entity_type: String,

    /// Entity description.
    pub description: String,

    /// Source document ID.
    pub source_id: String,

    /// Creation timestamp.
    pub created_at: String,

    /// Last update timestamp.
    pub updated_at: String,

    /// Node degree (number of connections).
    pub degree: usize,

    /// Additional metadata.
    pub metadata: serde_json::Value,
}

/// Create entity response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CreateEntityResponse {
    /// Operation status.
    pub status: String,

    /// Success message.
    pub message: String,

    /// Created entity.
    pub entity: EntityResponse,
}

/// Entity exists response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EntityExistsResponse {
    /// Whether the entity exists.
    pub exists: bool,

    /// Entity ID if exists.
    pub entity_id: Option<String>,

    /// Entity type if exists.
    pub entity_type: Option<String>,

    /// Node degree if exists.
    pub degree: Option<usize>,
}

/// Update entity response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UpdateEntityResponse {
    /// Operation status.
    pub status: String,

    /// Success message.
    pub message: String,

    /// Updated entity.
    pub entity: EntityResponse,

    /// Changes made.
    pub changes: ChangesSummary,
}

/// Delete entity response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DeleteEntityResponse {
    /// Operation status.
    pub status: String,

    /// Success message.
    pub message: String,

    /// Deleted entity ID.
    pub deleted_entity_id: String,

    /// Number of relationships deleted.
    pub deleted_relationships: usize,

    /// Affected entity IDs.
    pub affected_entities: Vec<String>,
}

/// Merge entities response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MergeEntitiesResponse {
    /// Operation status.
    pub status: String,

    /// Success message.
    pub message: String,

    /// Merged entity.
    pub merged_entity: EntityResponse,

    /// Merge details.
    pub merge_details: MergeDetails,
}

/// Merge operation details.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MergeDetails {
    /// Source entity ID.
    pub source_entity_id: String,

    /// Target entity ID.
    pub target_entity_id: String,

    /// Number of relationships merged.
    pub relationships_merged: usize,

    /// Number of duplicate relationships removed.
    pub duplicate_relationships_removed: usize,

    /// Description merge strategy used.
    pub description_strategy: String,

    /// Metadata merge strategy used.
    pub metadata_strategy: String,
}

/// Changes summary.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ChangesSummary {
    /// Fields that were updated.
    pub fields_updated: Vec<String>,

    /// Previous description if changed.
    pub previous_description: Option<String>,
}

/// Get entity with relationships response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GetEntityResponse {
    /// Entity data.
    pub entity: EntityResponse,

    /// Relationships.
    pub relationships: RelationshipsInfo,

    /// Statistics.
    pub statistics: EntityStatistics,
}

/// Relationships info.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RelationshipsInfo {
    /// Outgoing relationships.
    pub outgoing: Vec<RelationshipSummary>,

    /// Incoming relationships.
    pub incoming: Vec<RelationshipSummary>,
}

/// Relationship summary.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RelationshipSummary {
    /// Target entity ID (for outgoing) or source entity ID (for incoming).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,

    /// Source entity ID (for incoming).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// Relationship type.
    pub relation_type: String,

    /// Relationship weight.
    pub weight: f64,
}

/// Entity statistics.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EntityStatistics {
    /// Total relationships.
    pub total_relationships: usize,

    /// Outgoing relationships count.
    pub outgoing_count: usize,

    /// Incoming relationships count.
    pub incoming_count: usize,

    /// Document references count.
    pub document_references: usize,
}

/// Query parameters for entity neighborhood.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct EntityNeighborhoodQuery {
    /// Traversal depth (default 1, max 3).
    #[serde(default = "default_depth")]
    pub depth: u32,
}

/// Default traversal depth.
fn default_depth() -> u32 {
    1
}

/// Entity neighborhood response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EntityNeighborhoodResponse {
    /// Nodes in the neighborhood.
    pub nodes: Vec<NeighborhoodNode>,

    /// Edges between nodes.
    pub edges: Vec<NeighborhoodEdge>,
}

/// Node in the neighborhood graph.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NeighborhoodNode {
    /// Node ID (entity name).
    pub id: String,

    /// Entity type.
    pub entity_type: String,

    /// Entity description.
    pub description: String,

    /// Node degree (number of connections).
    pub degree: usize,
}

/// Edge in the neighborhood graph.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NeighborhoodEdge {
    /// Edge ID.
    pub id: String,

    /// Source node ID.
    pub source: String,

    /// Target node ID.
    pub target: String,

    /// Relationship type.
    pub relation_type: String,

    /// Edge weight.
    pub weight: f64,
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_merge_strategy() {
        assert_eq!(default_merge_strategy(), "prefer_target");
    }

    #[test]
    fn test_entities_default_true() {
        assert!(entities_default_true());
    }

    #[test]
    fn test_create_entity_request_deserialization() {
        let json = r#"{
            "entity_name": "Test Entity",
            "entity_type": "PERSON",
            "description": "A test entity",
            "source_id": "manual_entry"
        }"#;
        let req: CreateEntityRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.entity_name, "Test Entity");
        assert_eq!(req.entity_type, "PERSON");
        assert_eq!(req.source_id, "manual_entry");
    }

    #[test]
    fn test_merge_request_with_default_strategy() {
        let json = r#"{"source_entity": "ENTITY_A", "target_entity": "ENTITY_B"}"#;
        let req: MergeEntitiesRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.merge_strategy, "prefer_target");
    }

    #[test]
    fn test_delete_query_with_defaults() {
        let json = r#"{"confirm": true}"#;
        let query: DeleteEntityQuery = serde_json::from_str(json).unwrap();
        assert!(query.delete_relationships);
        assert!(query.confirm);
    }

    #[test]
    fn test_entity_exists_response_serialization() {
        let resp = EntityExistsResponse {
            exists: true,
            entity_id: Some("ENT_123".to_string()),
            entity_type: Some("PERSON".to_string()),
            degree: Some(5),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json["exists"].as_bool().unwrap());
        assert_eq!(json["entity_id"], "ENT_123");
    }

    #[test]
    fn test_entity_exists_response_not_found() {
        let resp = EntityExistsResponse {
            exists: false,
            entity_id: None,
            entity_type: None,
            degree: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(!json["exists"].as_bool().unwrap());
        assert!(json["entity_id"].is_null());
    }

    #[test]
    fn test_changes_summary_serialization() {
        let summary = ChangesSummary {
            fields_updated: vec!["description".to_string(), "metadata".to_string()],
            previous_description: Some("Old description".to_string()),
        };
        let json = serde_json::to_value(&summary).unwrap();
        assert_eq!(json["fields_updated"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_merge_details_serialization() {
        let details = MergeDetails {
            source_entity_id: "ENT_A".to_string(),
            target_entity_id: "ENT_B".to_string(),
            relationships_merged: 5,
            duplicate_relationships_removed: 2,
            description_strategy: "prefer_target".to_string(),
            metadata_strategy: "merge".to_string(),
        };
        let json = serde_json::to_value(&details).unwrap();
        assert_eq!(json["relationships_merged"], 5);
        assert_eq!(json["duplicate_relationships_removed"], 2);
    }

    #[test]
    fn test_entity_statistics_serialization() {
        let stats = EntityStatistics {
            total_relationships: 10,
            outgoing_count: 6,
            incoming_count: 4,
            document_references: 3,
        };
        let json = serde_json::to_value(&stats).unwrap();
        assert_eq!(json["total_relationships"], 10);
        assert_eq!(json["document_references"], 3);
    }

    #[test]
    fn test_relationship_summary_serialization() {
        let summary = RelationshipSummary {
            target: Some("ENT_TARGET".to_string()),
            source: None,
            relation_type: "WORKS_FOR".to_string(),
            weight: 0.85,
        };
        let json = serde_json::to_value(&summary).unwrap();
        assert_eq!(json["target"], "ENT_TARGET");
        assert!(json.get("source").is_none()); // skip_serializing_if works
        assert_eq!(json["relation_type"], "WORKS_FOR");
    }
}

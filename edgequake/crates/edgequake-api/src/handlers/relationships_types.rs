//! Relationship DTO types.
//!
//! This module contains all Data Transfer Objects for the relationship management API.
//! Extracted from relationships.rs for modularity and single responsibility.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ============================================================================
// Default value helper functions
// ============================================================================

/// Default relationship weight (0.8).
pub fn default_weight() -> f64 {
    0.8
}

// ============================================================================
// Request DTOs
// ============================================================================

/// Create relationship request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateRelationshipRequest {
    /// Source entity ID.
    pub src_id: String,

    /// Target entity ID.
    pub tgt_id: String,

    /// Keywords describing the relationship.
    pub keywords: String,

    /// Relationship weight (0.0 to 1.0).
    #[serde(default = "default_weight")]
    pub weight: f64,

    /// Relationship description.
    pub description: String,

    /// Source document ID (use "manual_entry" for manual entries).
    pub source_id: String,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Update relationship request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateRelationshipRequest {
    /// Updated keywords.
    pub keywords: Option<String>,

    /// Updated weight.
    pub weight: Option<f64>,

    /// Updated description.
    pub description: Option<String>,

    /// Updated metadata.
    pub metadata: Option<serde_json::Value>,
}

/// List relationships query parameters.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ListRelationshipsQuery {
    /// Page number (1-indexed).
    #[serde(default = "default_page")]
    pub page: u32,

    /// Page size (default 20, max 100).
    #[serde(default = "default_page_size")]
    pub page_size: u32,

    /// Filter by relationship type.
    pub relationship_type: Option<String>,
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

/// Paginated list of relationships response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListRelationshipsResponse {
    /// List of relationships.
    pub items: Vec<RelationshipResponse>,

    /// Total number of relationships matching the query.
    pub total: usize,

    /// Current page number.
    pub page: u32,

    /// Page size.
    pub page_size: u32,

    /// Total number of pages.
    pub total_pages: u32,
}

/// Relationship response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RelationshipResponse {
    /// Relationship ID.
    pub id: String,

    /// Source entity ID.
    pub src_id: String,

    /// Target entity ID.
    pub tgt_id: String,

    /// Relationship type.
    pub relation_type: String,

    /// Keywords.
    pub keywords: String,

    /// Weight.
    pub weight: f64,

    /// Description.
    pub description: String,

    /// Source document ID.
    pub source_id: String,

    /// Creation timestamp.
    pub created_at: String,

    /// Last update timestamp.
    pub updated_at: String,

    /// Additional metadata.
    pub metadata: serde_json::Value,
}

/// Create relationship response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CreateRelationshipResponse {
    /// Operation status.
    pub status: String,

    /// Success message.
    pub message: String,

    /// Created relationship.
    pub relationship: RelationshipResponse,
}

/// Get relationship response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GetRelationshipResponse {
    /// Relationship data.
    pub relationship: RelationshipResponse,

    /// Entities involved.
    pub entities: RelationshipEntities,
}

/// Entities in a relationship.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RelationshipEntities {
    /// Source entity.
    pub source: EntitySummary,

    /// Target entity.
    pub target: EntitySummary,
}

/// Entity summary.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EntitySummary {
    /// Entity ID.
    pub id: String,

    /// Entity type.
    pub entity_type: String,
}

/// Update relationship response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UpdateRelationshipResponse {
    /// Operation status.
    pub status: String,

    /// Success message.
    pub message: String,

    /// Updated relationship.
    pub relationship: RelationshipResponse,

    /// Changes made.
    pub changes: RelationshipChangesSummary,
}

/// Relationship changes summary.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RelationshipChangesSummary {
    /// Fields that were updated.
    pub fields_updated: Vec<String>,

    /// Previous weight if changed.
    pub previous_weight: Option<f64>,
}

/// Delete relationship response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DeleteRelationshipResponse {
    /// Operation status.
    pub status: String,

    /// Success message.
    pub message: String,

    /// Deleted relationship ID.
    pub deleted_relationship_id: String,

    /// Source entity ID.
    pub src_id: String,

    /// Target entity ID.
    pub tgt_id: String,
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_weight() {
        assert_eq!(default_weight(), 0.8);
    }

    #[test]
    fn test_create_relationship_request() {
        let json = r#"{
            "src_id": "ENTITY_A",
            "tgt_id": "ENTITY_B",
            "keywords": "collaborates_with",
            "description": "Research collaboration",
            "source_id": "doc_123"
        }"#;
        let req: CreateRelationshipRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.src_id, "ENTITY_A");
        assert_eq!(req.weight, 0.8); // default
        assert_eq!(req.keywords, "collaborates_with");
    }

    #[test]
    fn test_create_relationship_request_custom_weight() {
        let json = r#"{
            "src_id": "ENTITY_A",
            "tgt_id": "ENTITY_B",
            "keywords": "manages",
            "weight": 0.95,
            "description": "Management relationship",
            "source_id": "manual_entry",
            "metadata": {"verified": true}
        }"#;
        let req: CreateRelationshipRequest = serde_json::from_str(json).unwrap();
        let weight = req.weight;
        assert!((weight - 0.95).abs() < 0.001);
    }

    #[test]
    fn test_update_relationship_request() {
        let json = r#"{"keywords": "updated_relation", "weight": 0.9}"#;
        let req: UpdateRelationshipRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.keywords, Some("updated_relation".to_string()));
        let weight = req.weight.unwrap();
        assert!((weight - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_relationship_response() {
        let response = RelationshipResponse {
            id: "rel_123".to_string(),
            src_id: "ENT_A".to_string(),
            tgt_id: "ENT_B".to_string(),
            relation_type: "collaborates".to_string(),
            keywords: "works_with".to_string(),
            weight: 0.85,
            description: "Collaboration".to_string(),
            source_id: "doc_456".to_string(),
            created_at: "2026-01-07T12:00:00Z".to_string(),
            updated_at: "2026-01-07T12:00:00Z".to_string(),
            metadata: serde_json::json!({}),
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["id"], "rel_123");
        assert_eq!(json["src_id"], "ENT_A");
    }

    #[test]
    fn test_entity_summary() {
        let entity = EntitySummary {
            id: "ENT_123".to_string(),
            entity_type: "PERSON".to_string(),
        };
        let json = serde_json::to_value(&entity).unwrap();
        assert_eq!(json["id"], "ENT_123");
        assert_eq!(json["entity_type"], "PERSON");
    }

    #[test]
    fn test_relationship_entities() {
        let entities = RelationshipEntities {
            source: EntitySummary {
                id: "ENT_A".to_string(),
                entity_type: "PERSON".to_string(),
            },
            target: EntitySummary {
                id: "ENT_B".to_string(),
                entity_type: "ORGANIZATION".to_string(),
            },
        };
        let json = serde_json::to_value(&entities).unwrap();
        assert_eq!(json["source"]["id"], "ENT_A");
        assert_eq!(json["target"]["entity_type"], "ORGANIZATION");
    }

    #[test]
    fn test_create_relationship_response() {
        let response = CreateRelationshipResponse {
            status: "success".to_string(),
            message: "Relationship created".to_string(),
            relationship: RelationshipResponse {
                id: "rel_456".to_string(),
                src_id: "ENT_A".to_string(),
                tgt_id: "ENT_B".to_string(),
                relation_type: "related".to_string(),
                keywords: "test".to_string(),
                weight: 0.8,
                description: "Test".to_string(),
                source_id: "doc".to_string(),
                created_at: "2026-01-07T00:00:00Z".to_string(),
                updated_at: "2026-01-07T00:00:00Z".to_string(),
                metadata: serde_json::json!({}),
            },
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["status"], "success");
        assert_eq!(json["relationship"]["id"], "rel_456");
    }

    #[test]
    fn test_relationship_changes_summary() {
        let changes = RelationshipChangesSummary {
            fields_updated: vec!["keywords".to_string(), "weight".to_string()],
            previous_weight: Some(0.7),
        };
        let json = serde_json::to_value(&changes).unwrap();
        assert_eq!(json["fields_updated"][0], "keywords");
        let prev_weight = json["previous_weight"].as_f64().unwrap();
        assert!((prev_weight - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_delete_relationship_response() {
        let response = DeleteRelationshipResponse {
            status: "success".to_string(),
            message: "Deleted".to_string(),
            deleted_relationship_id: "rel_789".to_string(),
            src_id: "ENT_X".to_string(),
            tgt_id: "ENT_Y".to_string(),
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["status"], "success");
        assert_eq!(json["deleted_relationship_id"], "rel_789");
    }
}

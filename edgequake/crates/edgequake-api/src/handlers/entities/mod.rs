//! Entity CRUD operations for manual knowledge graph management.
//!
//! # Implements
//!
//! - **UC0101**: Explore Entity Neighborhood
//! - **UC0102**: Search Entities by Name
//! - **UC0103**: Delete Entity from Graph
//! - **FEAT0002**: Entity Extraction (view extracted entities)
//! - **FEAT0202**: Graph Traversal
//! - **FEAT0203**: Graph Mutation Operations
//! - **FEAT0401**: REST API Service
//!
//! # Enforces
//!
//! - **BR0008**: Entity names normalized (UPPERCASE with underscores)
//! - **BR0005**: Entity description max 512 tokens
//! - **BR0201**: Tenant isolation
//!
//! # Endpoints
//!
//! | Method | Path | Handler | Description |
//! |--------|------|---------|-------------|
//! | GET | `/api/v1/graph/entities` | [`list_entities`] | List with pagination |
//! | GET | `/api/v1/graph/entities/:id` | [`get_entity`] | Get single entity |
//! | POST | `/api/v1/graph/entities` | [`create_entity`] | Manually create entity |
//! | PUT | `/api/v1/graph/entities/:id` | [`update_entity`] | Update entity |
//! | DELETE | `/api/v1/graph/entities/:id` | [`delete_entity`] | Delete with cascade |
//! | GET | `/api/v1/graph/entities/:id/neighbors` | [`get_entity_neighbors`] | Get connected entities |
//!
//! # WHY: Manual Entity Management
//!
//! While entities are typically extracted automatically from documents, users need
//! manual CRUD operations for:
//! - Correcting extraction errors
//! - Adding domain knowledge not in documents
//! - Merging duplicate entities
//! - Curating the knowledge graph

mod entity_crud;
mod entity_ops;

pub use entity_crud::*;
pub use entity_ops::*;

// Re-export DTOs from entities_types module
pub use crate::handlers::entities_types::*;

use crate::services::entity_name_normalize;
use edgequake_storage::GraphNode;

// ============================================================================
// Shared Helper Functions
// ============================================================================

/// Normalize entity name using the API SSOT (delegates to storage canonical form).
pub(super) fn normalize_entity_name_for_graph(name: &str) -> String {
    entity_name_normalize::normalize_entity_name(name)
}

/// Convert GraphNode to EntityResponse.
pub(super) fn node_to_entity_response(node: GraphNode, degree: usize) -> EntityResponse {
    let props = &node.properties;

    EntityResponse {
        id: node.id.clone(),
        entity_name: node.id.clone(),
        entity_type: props
            .get("entity_type")
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN")
            .to_string(),
        description: props
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        source_id: props
            .get("source_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        created_at: props
            .get("created_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        updated_at: props
            .get("updated_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        degree,
        metadata: props
            .get("metadata")
            .cloned()
            .unwrap_or(serde_json::json!({})),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_entity_name() {
        assert_eq!(
            normalize_entity_name_for_graph("quantum computing"),
            "QUANTUM_COMPUTING"
        );
        assert_eq!(normalize_entity_name_for_graph("AI"), "AI");
        assert_eq!(
            normalize_entity_name_for_graph("Machine Learning"),
            "MACHINE_LEARNING"
        );
    }

    #[test]
    fn test_normalize_entity_name_edge_cases() {
        // Single space replaced with underscore
        assert_eq!(
            normalize_entity_name_for_graph("hello world"),
            "HELLO_WORLD"
        );
        // Canonical normalizer collapses runs of whitespace (split_whitespace)
        assert_eq!(
            normalize_entity_name_for_graph("hello  world"),
            "HELLO_WORLD"
        );
        // Empty string
        assert_eq!(normalize_entity_name_for_graph(""), "");
        // Already uppercase
        assert_eq!(
            normalize_entity_name_for_graph("ALREADY UPPERCASE"),
            "ALREADY_UPPERCASE"
        );
    }

    #[test]
    fn test_create_entity_request_deserialization() {
        let json = r#"{
            "entity_name": "test entity",
            "entity_type": "CONCEPT",
            "description": "A test entity",
            "source_id": "manual_entry"
        }"#;
        let request: Result<CreateEntityRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert_eq!(req.entity_name, "test entity");
        assert_eq!(req.entity_type, "CONCEPT");
    }

    #[test]
    fn test_update_entity_request_partial() {
        // Only description
        let json = r#"{"description": "Updated description"}"#;
        let request: Result<UpdateEntityRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert!(req.entity_type.is_none());
        assert_eq!(req.description, Some("Updated description".to_string()));
    }

    #[test]
    fn test_entity_response_serialization() {
        let response = EntityResponse {
            id: "test-id".to_string(),
            entity_name: "TEST_ENTITY".to_string(),
            entity_type: "CONCEPT".to_string(),
            description: "A test".to_string(),
            source_id: "doc-1".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            degree: 5,
            metadata: serde_json::Value::Null,
        };
        let json = serde_json::to_string(&response);
        assert!(json.is_ok());
        assert!(json.unwrap().contains("TEST_ENTITY"));
    }

    #[test]
    fn test_merge_entities_request_deserialization() {
        let json = r#"{
            "source_entity": "ENTITY_A",
            "target_entity": "ENTITY_B"
        }"#;
        let request: Result<MergeEntitiesRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert_eq!(req.source_entity, "ENTITY_A");
        assert_eq!(req.target_entity, "ENTITY_B");
    }

    #[test]
    fn test_delete_entity_query_deserialization() {
        let json = r#"{"delete_relationships": true, "confirm": true}"#;
        let query: Result<DeleteEntityQuery, _> = serde_json::from_str(json);
        assert!(query.is_ok());
        let q = query.unwrap();
        assert!(q.delete_relationships);
        assert!(q.confirm);
    }

    #[test]
    fn test_entity_statistics_serialization() {
        let stats = EntityStatistics {
            total_relationships: 100,
            outgoing_count: 50,
            incoming_count: 50,
            document_references: 10,
        };
        let json = serde_json::to_string(&stats);
        assert!(json.is_ok());
    }
}

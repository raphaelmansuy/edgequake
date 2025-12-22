# Graph Management & CRUD Operations

**Specification Version:** 1.0  
**Target Release:** EdgeQuake v1.2.0  
**Priority:** MEDIUM  
**Status:** Planning

---

## Overview

Enable manual knowledge graph management through CRUD operations on entities and relationships, allowing users to correct, enhance, and curate the automatically extracted knowledge graph.

### Goals

1. **Entity Management:** Create, read, update, delete, merge entities
2. **Relationship Management:** Create, read, update, delete relationships
3. **Entity Deduplication:** Merge duplicate entities intelligently
4. **Graph Analytics:** Enhanced statistics and insights
5. **Validation:** Ensure graph integrity and consistency
6. **Audit Trail:** Track manual changes to the graph

---

## API Endpoints

### Entity Operations

#### 1. Create Entity

```http
POST /api/v1/graph/entities
Content-Type: application/json
```

**Request:**
```json
{
  "entity_name": "QUANTUM_COMPUTING",
  "entity_type": "TECHNOLOGY",
  "description": "Computing paradigm that uses quantum mechanics principles",
  "source_id": "manual_entry_123",
  "metadata": {
    "created_by": "user@example.com",
    "confidence": "high",
    "tags": ["technology", "computing"]
  }
}
```

**Response (201 Created):**
```json
{
  "status": "success",
  "message": "Entity created successfully",
  "entity": {
    "id": "QUANTUM_COMPUTING",
    "entity_name": "QUANTUM_COMPUTING",
    "entity_type": "TECHNOLOGY",
    "description": "Computing paradigm that uses quantum mechanics principles",
    "source_id": "manual_entry_123",
    "created_at": "2025-12-22T19:00:00Z",
    "updated_at": "2025-12-22T19:00:00Z",
    "degree": 0,
    "metadata": {
      "created_by": "user@example.com",
      "confidence": "high",
      "tags": ["technology", "computing"]
    }
  }
}
```

**Response (409 Conflict):**
```json
{
  "error": "entity_exists",
  "message": "Entity 'QUANTUM_COMPUTING' already exists",
  "existing_entity_id": "QUANTUM_COMPUTING",
  "suggestion": "Use PUT /api/v1/graph/entities/QUANTUM_COMPUTING to update"
}
```

#### 2. Get Entity

```http
GET /api/v1/graph/entities/{entity_name}
```

**Response (200 OK):**
```json
{
  "entity": {
    "id": "QUANTUM_COMPUTING",
    "entity_name": "QUANTUM_COMPUTING",
    "entity_type": "TECHNOLOGY",
    "description": "Computing paradigm that uses quantum mechanics principles",
    "source_id": "manual_entry_123",
    "created_at": "2025-12-22T19:00:00Z",
    "updated_at": "2025-12-22T19:00:00Z",
    "degree": 15,
    "metadata": {}
  },
  "relationships": {
    "outgoing": [
      {
        "target": "CRYPTOGRAPHY",
        "relation_type": "APPLIES_TO",
        "weight": 0.95
      }
    ],
    "incoming": [
      {
        "source": "QUANTUM_MECHANICS",
        "relation_type": "FOUNDATION_OF",
        "weight": 0.98
      }
    ]
  },
  "statistics": {
    "total_relationships": 15,
    "outgoing_count": 8,
    "incoming_count": 7,
    "document_references": 42
  }
}
```

#### 3. Update Entity

```http
PUT /api/v1/graph/entities/{entity_name}
Content-Type: application/json
```

**Request:**
```json
{
  "entity_type": "TECHNOLOGY",
  "description": "Updated: Advanced computing paradigm leveraging quantum mechanics for computational advantages",
  "metadata": {
    "updated_by": "admin@example.com",
    "update_reason": "improved_description",
    "confidence": "very_high"
  }
}
```

**Response (200 OK):**
```json
{
  "status": "success",
  "message": "Entity updated successfully",
  "entity": {
    "id": "QUANTUM_COMPUTING",
    "entity_name": "QUANTUM_COMPUTING",
    "entity_type": "TECHNOLOGY",
    "description": "Updated: Advanced computing paradigm...",
    "updated_at": "2025-12-22T19:05:00Z",
    "metadata": {}
  },
  "changes": {
    "fields_updated": ["description", "metadata"],
    "previous_description": "Computing paradigm that uses..."
  }
}
```

#### 4. Delete Entity

```http
DELETE /api/v1/graph/entities/{entity_name}
```

**Query Parameters:**
- `delete_relationships`: boolean (default: true) - Also delete connected relationships
- `confirm`: boolean (required: true) - Confirmation flag

**Response (200 OK):**
```json
{
  "status": "success",
  "message": "Entity deleted successfully",
  "deleted_entity_id": "QUANTUM_COMPUTING",
  "deleted_relationships": 15,
  "affected_entities": ["CRYPTOGRAPHY", "QUANTUM_MECHANICS", "..."]
}
```

#### 5. Check Entity Exists

```http
GET /api/v1/graph/entities/exists?entity_name=QUANTUM_COMPUTING
```

**Response (200 OK):**
```json
{
  "exists": true,
  "entity_id": "QUANTUM_COMPUTING",
  "entity_type": "TECHNOLOGY",
  "degree": 15
}
```

#### 6. Merge Entities (Deduplication)

```http
POST /api/v1/graph/entities/merge
Content-Type: application/json
```

**Request:**
```json
{
  "source_entity": "quantum_computing",
  "target_entity": "QUANTUM_COMPUTING",
  "merge_strategy": "prefer_target",
  "metadata": {
    "reason": "duplicate_normalization",
    "merged_by": "admin@example.com"
  }
}
```

**Response (200 OK):**
```json
{
  "status": "success",
  "message": "Entities merged successfully",
  "merged_entity": {
    "id": "QUANTUM_COMPUTING",
    "entity_name": "QUANTUM_COMPUTING",
    "entity_type": "TECHNOLOGY",
    "description": "Combined description...",
    "degree": 23
  },
  "merge_details": {
    "source_entity_id": "quantum_computing",
    "target_entity_id": "QUANTUM_COMPUTING",
    "relationships_merged": 8,
    "duplicate_relationships_removed": 2,
    "description_strategy": "prefer_target",
    "metadata_strategy": "merge"
  }
}
```

### Relationship Operations

#### 7. Create Relationship

```http
POST /api/v1/graph/relationships
Content-Type: application/json
```

**Request:**
```json
{
  "src_id": "QUANTUM_COMPUTING",
  "tgt_id": "CRYPTOGRAPHY",
  "keywords": "application, security, encryption",
  "weight": 0.95,
  "description": "Quantum computing has significant applications in cryptography, particularly for breaking classical encryption",
  "source_id": "manual_entry_456",
  "metadata": {
    "created_by": "expert@example.com",
    "confidence": "high",
    "evidence": ["research_paper_1", "textbook_2"]
  }
}
```

**Response (201 Created):**
```json
{
  "status": "success",
  "message": "Relationship created successfully",
  "relationship": {
    "id": "rel-abc123",
    "src_id": "QUANTUM_COMPUTING",
    "tgt_id": "CRYPTOGRAPHY",
    "relation_type": "APPLIES_TO",
    "keywords": "application, security, encryption",
    "weight": 0.95,
    "description": "Quantum computing has significant applications...",
    "source_id": "manual_entry_456",
    "created_at": "2025-12-22T19:10:00Z",
    "metadata": {}
  }
}
```

#### 8. Get Relationship

```http
GET /api/v1/graph/relationships/{relationship_id}
```

**Response (200 OK):**
```json
{
  "relationship": {
    "id": "rel-abc123",
    "src_id": "QUANTUM_COMPUTING",
    "tgt_id": "CRYPTOGRAPHY",
    "relation_type": "APPLIES_TO",
    "keywords": "application, security, encryption",
    "weight": 0.95,
    "description": "Quantum computing has significant applications...",
    "created_at": "2025-12-22T19:10:00Z",
    "updated_at": "2025-12-22T19:10:00Z",
    "metadata": {}
  },
  "entities": {
    "source": {
      "id": "QUANTUM_COMPUTING",
      "entity_type": "TECHNOLOGY"
    },
    "target": {
      "id": "CRYPTOGRAPHY",
      "entity_type": "FIELD"
    }
  }
}
```

#### 9. Update Relationship

```http
PUT /api/v1/graph/relationships/{relationship_id}
Content-Type: application/json
```

**Request:**
```json
{
  "keywords": "application, security, encryption, quantum_resistance",
  "weight": 0.98,
  "description": "Updated: Quantum computing revolutionizes cryptography...",
  "metadata": {
    "updated_by": "expert@example.com",
    "update_reason": "new_research_findings"
  }
}
```

**Response (200 OK):**
```json
{
  "status": "success",
  "message": "Relationship updated successfully",
  "relationship": {
    "id": "rel-abc123",
    "src_id": "QUANTUM_COMPUTING",
    "tgt_id": "CRYPTOGRAPHY",
    "weight": 0.98,
    "updated_at": "2025-12-22T19:15:00Z"
  },
  "changes": {
    "fields_updated": ["keywords", "weight", "description"],
    "previous_weight": 0.95
  }
}
```

#### 10. Delete Relationship

```http
DELETE /api/v1/graph/relationships/{relationship_id}
```

**Response (200 OK):**
```json
{
  "status": "success",
  "message": "Relationship deleted successfully",
  "deleted_relationship_id": "rel-abc123",
  "src_id": "QUANTUM_COMPUTING",
  "tgt_id": "CRYPTOGRAPHY"
}
```

### Graph Analytics

#### 11. Get Graph Statistics

```http
GET /api/v1/graph/statistics
```

**Response (200 OK):**
```json
{
  "nodes": {
    "total": 50000,
    "by_type": {
      "PERSON": 12000,
      "ORGANIZATION": 8000,
      "LOCATION": 5000,
      "TECHNOLOGY": 3000,
      "CONCEPT": 22000
    }
  },
  "edges": {
    "total": 125000,
    "by_type": {
      "WORKS_FOR": 15000,
      "LOCATED_IN": 8000,
      "RELATED_TO": 50000,
      "APPLIES_TO": 12000,
      "OTHER": 40000
    },
    "avg_weight": 0.75
  },
  "connectivity": {
    "avg_degree": 5.0,
    "max_degree": 250,
    "connected_components": 5,
    "largest_component_size": 49000
  },
  "quality": {
    "entities_with_descriptions": 45000,
    "description_coverage_percent": 90.0,
    "manual_entries": 150,
    "auto_extracted": 49850
  }
}
```

#### 12. Get Popular Labels

```http
GET /api/v1/graph/labels/popular?limit=20
```

**Response (200 OK):**
```json
{
  "labels": [
    {
      "entity_name": "ARTIFICIAL_INTELLIGENCE",
      "entity_type": "TECHNOLOGY",
      "degree": 250,
      "document_references": 450
    },
    {
      "entity_name": "MACHINE_LEARNING",
      "entity_type": "TECHNOLOGY",
      "degree": 230,
      "document_references": 420
    }
  ],
  "total": 20
}
```

#### 13. Search Labels

```http
GET /api/v1/graph/labels/search?q=quantum&limit=10
```

**Response (200 OK):**
```json
{
  "results": [
    {
      "entity_name": "QUANTUM_COMPUTING",
      "entity_type": "TECHNOLOGY",
      "description": "Computing paradigm...",
      "degree": 15,
      "relevance_score": 0.98
    },
    {
      "entity_name": "QUANTUM_MECHANICS",
      "entity_type": "FIELD",
      "description": "Branch of physics...",
      "degree": 22,
      "relevance_score": 0.95
    }
  ],
  "total": 2,
  "query": "quantum"
}
```

---

## Data Model

### Entity Schema (Extended)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub entity_name: String,
    pub entity_type: String,
    pub description: String,
    pub source_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_manual: bool,  // True if manually created
    pub metadata: serde_json::Value,
}

// AGE Cypher query to create entity
CREATE (e:Entity {
    id: $id,
    entity_name: $entity_name,
    entity_type: $entity_type,
    description: $description,
    source_id: $source_id,
    created_at: $created_at,
    updated_at: $updated_at,
    is_manual: $is_manual,
    metadata: $metadata
})
```

### Relationship Schema (Extended)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub id: String,
    pub src_id: String,
    pub tgt_id: String,
    pub relation_type: String,
    pub keywords: String,
    pub weight: f64,
    pub description: String,
    pub source_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_manual: bool,
    pub metadata: serde_json::Value,
}

// AGE Cypher query to create relationship
MATCH (src:Entity {id: $src_id})
MATCH (tgt:Entity {id: $tgt_id})
CREATE (src)-[r:RELATION {
    id: $id,
    relation_type: $relation_type,
    keywords: $keywords,
    weight: $weight,
    description: $description,
    source_id: $source_id,
    created_at: $created_at,
    updated_at: $updated_at,
    is_manual: $is_manual,
    metadata: $metadata
}]->(tgt)
```

### Audit Log Schema

```sql
CREATE TABLE graph_audit_log (
    id SERIAL PRIMARY KEY,
    operation VARCHAR(20) NOT NULL,  -- create, update, delete, merge
    entity_type VARCHAR(20) NOT NULL,  -- entity, relationship
    entity_id VARCHAR(200) NOT NULL,
    user_id VARCHAR(100),
    changes JSONB,
    reason TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT valid_operation CHECK (operation IN ('create', 'update', 'delete', 'merge'))
);

CREATE INDEX idx_audit_entity ON graph_audit_log(entity_id, created_at DESC);
CREATE INDEX idx_audit_user ON graph_audit_log(user_id, created_at DESC);
```

---

## Implementation

### Entity Merge Logic

```rust
pub struct EntityMerger {
    graph_storage: Arc<dyn GraphStorage>,
    audit_log: Arc<dyn AuditLog>,
}

impl EntityMerger {
    pub async fn merge_entities(
        &self,
        source_id: &str,
        target_id: &str,
        strategy: MergeStrategy,
        user_id: Option<&str>,
    ) -> Result<Entity, Error> {
        // 1. Get both entities
        let source = self.graph_storage.get_entity(source_id).await?
            .ok_or(Error::EntityNotFound(source_id.to_string()))?;
        let target = self.graph_storage.get_entity(target_id).await?
            .ok_or(Error::EntityNotFound(target_id.to_string()))?;
        
        // 2. Merge descriptions
        let merged_description = match strategy {
            MergeStrategy::PreferTarget => target.description,
            MergeStrategy::PreferSource => source.description,
            MergeStrategy::Concatenate => {
                format!("{}. {}", target.description, source.description)
            }
            MergeStrategy::LongerDescription => {
                if target.description.len() > source.description.len() {
                    target.description
                } else {
                    source.description
                }
            }
        };
        
        // 3. Get all relationships of source entity
        let source_rels = self.graph_storage.get_entity_relationships(source_id).await?;
        
        // 4. Redirect relationships to target entity
        for rel in source_rels {
            // Check if similar relationship already exists
            if self.relationship_exists(&rel, target_id).await? {
                // Delete duplicate
                self.graph_storage.delete_relationship(&rel.id).await?;
            } else {
                // Update relationship to point to target
                self.graph_storage.update_relationship_endpoint(
                    &rel.id,
                    source_id,
                    target_id,
                ).await?;
            }
        }
        
        // 5. Update target entity
        let mut merged_entity = target.clone();
        merged_entity.description = merged_description;
        merged_entity.updated_at = Utc::now();
        
        self.graph_storage.update_entity(&merged_entity).await?;
        
        // 6. Delete source entity
        self.graph_storage.delete_entity(source_id).await?;
        
        // 7. Log merge operation
        self.audit_log.log_merge(
            source_id,
            target_id,
            user_id,
            &format!("Merged {} into {}", source_id, target_id),
        ).await?;
        
        Ok(merged_entity)
    }
}
```

### Graph Validation

```rust
pub struct GraphValidator {
    graph_storage: Arc<dyn GraphStorage>,
}

impl GraphValidator {
    pub async fn validate_entity_create(
        &self,
        entity: &Entity,
    ) -> Result<(), ValidationError> {
        // Check entity name is normalized
        if entity.entity_name != entity.entity_name.to_uppercase() {
            return Err(ValidationError::InvalidEntityName(
                "Entity name must be uppercase".to_string()
            ));
        }
        
        // Check entity doesn't already exist
        if self.graph_storage.get_entity(&entity.id).await?.is_some() {
            return Err(ValidationError::EntityExists(entity.id.clone()));
        }
        
        // Check description is not empty
        if entity.description.trim().is_empty() {
            return Err(ValidationError::EmptyDescription);
        }
        
        Ok(())
    }
    
    pub async fn validate_relationship_create(
        &self,
        relationship: &Relationship,
    ) -> Result<(), ValidationError> {
        // Check source entity exists
        if self.graph_storage.get_entity(&relationship.src_id).await?.is_none() {
            return Err(ValidationError::EntityNotFound(relationship.src_id.clone()));
        }
        
        // Check target entity exists
        if self.graph_storage.get_entity(&relationship.tgt_id).await?.is_none() {
            return Err(ValidationError::EntityNotFound(relationship.tgt_id.clone()));
        }
        
        // Check weight is in valid range [0, 1]
        if relationship.weight < 0.0 || relationship.weight > 1.0 {
            return Err(ValidationError::InvalidWeight(relationship.weight));
        }
        
        Ok(())
    }
}
```

---

## Testing

```rust
#[tokio::test]
async fn test_create_entity() {
    let app = test_app().await;
    
    let response = app
        .post("/api/v1/graph/entities")
        .json(&json!({
            "entity_name": "TEST_ENTITY",
            "entity_type": "CONCEPT",
            "description": "Test entity description",
            "source_id": "test"
        }))
        .send()
        .await;
    
    assert_eq!(response.status(), StatusCode::CREATED);
    
    let body: CreateEntityResponse = response.json().await;
    assert_eq!(body.entity.entity_name, "TEST_ENTITY");
}

#[tokio::test]
async fn test_merge_entities() {
    let app = test_app().await;
    
    // Create two entities
    create_entity(&app, "ENTITY_A").await;
    create_entity(&app, "ENTITY_B").await;
    
    // Merge them
    let response = app
        .post("/api/v1/graph/entities/merge")
        .json(&json!({
            "source_entity": "ENTITY_A",
            "target_entity": "ENTITY_B",
            "merge_strategy": "prefer_target"
        }))
        .send()
        .await;
    
    assert_eq!(response.status(), StatusCode::OK);
    
    // Verify source entity is deleted
    let get_response = app
        .get("/api/v1/graph/entities/ENTITY_A")
        .send()
        .await;
    assert_eq!(get_response.status(), StatusCode::NOT_FOUND);
}
```

---

**Status:** ✅ Specification Complete  
**Dependencies:** AGE graph storage  
**Next:** Implement entity CRUD handlers and merge logic

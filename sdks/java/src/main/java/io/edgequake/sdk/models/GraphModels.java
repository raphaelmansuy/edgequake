package io.edgequake.sdk.models;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.List;
import java.util.Map;

/**
 * Graph-related model classes: Entity, Relationship, GraphNode, GraphEdge.
 *
 * WHY: All field names match the real EdgeQuake API (verified via curl).
 * entity_name (not name), source_id (not sourceId), etc.
 */
public class GraphModels {

    // ── Entity ────────────────────────────────────────────────────────

    public static class Entity {
        @JsonProperty("id") public String id;
        @JsonProperty("entity_name") public String entityName;
        @JsonProperty("name") public String name;
        @JsonProperty("entity_type") public String entityType;
        @JsonProperty("description") public String description;
        @JsonProperty("source_id") public String sourceId;
        @JsonProperty("degree") public Integer degree;
        @JsonProperty("metadata") public Object metadata;
        @JsonProperty("created_at") public String createdAt;
        @JsonProperty("updated_at") public String updatedAt;
    }

    /** WHY: All four fields are REQUIRED by the real API. */
    public static class CreateEntityRequest {
        @JsonProperty("entity_name") public String entityName;
        @JsonProperty("entity_type") public String entityType;
        @JsonProperty("description") public String description;
        @JsonProperty("source_id") public String sourceId;
        @JsonProperty("metadata") public Object metadata;

        public CreateEntityRequest() {}
        public CreateEntityRequest(String entityName, String entityType,
                                   String description, String sourceId) {
            this.entityName = entityName;
            this.entityType = entityType;
            this.description = description;
            this.sourceId = sourceId;
        }
    }

    /** Response from POST /api/v1/graph/entities. */
    public static class CreateEntityResponse {
        @JsonProperty("status") public String status;
        @JsonProperty("message") public String message;
        @JsonProperty("entity") public Entity entity;
    }

    /**
     * Response from GET /api/v1/graph/entities/{name}.
     * WHY: Returns entity + relationships + statistics wrapper.
     */
    public static class EntityDetailResponse {
        @JsonProperty("entity") public Entity entity;
        @JsonProperty("relationships") public EntityRelationships relationships;
        @JsonProperty("statistics") public EntityStatistics statistics;
    }

    public static class EntityRelationships {
        @JsonProperty("outgoing") public List<Relationship> outgoing;
        @JsonProperty("incoming") public List<Relationship> incoming;
    }

    public static class EntityStatistics {
        @JsonProperty("total_relationships") public int totalRelationships;
        @JsonProperty("outgoing_count") public int outgoingCount;
        @JsonProperty("incoming_count") public int incomingCount;
        @JsonProperty("document_references") public int documentReferences;
    }

    /** Paginated entity list: {items, total, page, page_size, total_pages}. */
    public static class EntityListResponse {
        @JsonProperty("items") public List<Entity> items;
        @JsonProperty("total") public int total;
        @JsonProperty("page") public int page;
        @JsonProperty("page_size") public int pageSize;
        @JsonProperty("total_pages") public int totalPages;
    }

    /** WHY: entity_id (not entity_name) in exists response. */
    public static class EntityExistsResponse {
        @JsonProperty("exists") public boolean exists;
        @JsonProperty("entity_id") public String entityId;
        @JsonProperty("entity_type") public String entityType;
        @JsonProperty("degree") public Integer degree;
    }

    /** WHY: Uses source_entity/target_entity (not source/target). */
    public static class MergeEntitiesRequest {
        @JsonProperty("source_entity") public String sourceEntity;
        @JsonProperty("target_entity") public String targetEntity;

        public MergeEntitiesRequest() {}
        public MergeEntitiesRequest(String sourceEntity, String targetEntity) {
            this.sourceEntity = sourceEntity;
            this.targetEntity = targetEntity;
        }
    }

    public static class MergeResponse {
        @JsonProperty("merged_entity") public Entity mergedEntity;
        @JsonProperty("merged_count") public int mergedCount;
        @JsonProperty("message") public String message;
    }

    /** Entity delete response. WHY: Requires ?confirm=true query param. */
    public static class EntityDeleteResponse {
        @JsonProperty("status") public String status;
        @JsonProperty("message") public String message;
        @JsonProperty("deleted_entity_id") public String deletedEntityId;
        @JsonProperty("deleted_relationships") public int deletedRelationships;
        @JsonProperty("affected_entities") public java.util.List<String> affectedEntities;
    }

    // ── Relationship ─────────────────────────────────────────────────

    public static class Relationship {
        @JsonProperty("id") public String id;
        @JsonProperty("source") public String source;
        @JsonProperty("target") public String target;
        @JsonProperty("relationship_type") public String relationshipType;
        @JsonProperty("weight") public Double weight;
        @JsonProperty("description") public String description;
        @JsonProperty("properties") public Map<String, Object> properties;
    }

    public static class CreateRelationshipRequest {
        @JsonProperty("source") public String source;
        @JsonProperty("target") public String target;
        @JsonProperty("relationship_type") public String relationshipType;
        @JsonProperty("weight") public Double weight;
        @JsonProperty("description") public String description;

        public CreateRelationshipRequest() {}
        public CreateRelationshipRequest(String source, String target,
                                         String relationshipType) {
            this.source = source;
            this.target = target;
            this.relationshipType = relationshipType;
        }
    }

    /** Paginated relationship list. */
    public static class RelationshipListResponse {
        @JsonProperty("items") public List<Relationship> items;
        @JsonProperty("total") public int total;
        @JsonProperty("page") public int page;
        @JsonProperty("page_size") public int pageSize;
        @JsonProperty("total_pages") public int totalPages;
    }

    // ── Graph ────────────────────────────────────────────────────────

    public static class GraphNode {
        @JsonProperty("id") public String id;
        @JsonProperty("label") public String label;
        @JsonProperty("node_type") public String nodeType;
        @JsonProperty("description") public String description;
        @JsonProperty("properties") public Map<String, Object> properties;
        @JsonProperty("degree") public Integer degree;
    }

    public static class GraphEdge {
        @JsonProperty("source") public String source;
        @JsonProperty("target") public String target;
        @JsonProperty("edge_type") public String edgeType;
        @JsonProperty("weight") public Double weight;
        @JsonProperty("properties") public Map<String, Object> properties;
    }

    public static class GraphResponse {
        @JsonProperty("nodes") public List<GraphNode> nodes;
        @JsonProperty("edges") public List<GraphEdge> edges;
        @JsonProperty("total_nodes") public Integer totalNodes;
        @JsonProperty("total_edges") public Integer totalEdges;
    }

    /** WHY: Search uses /api/v1/graph/nodes/search?q=... per routes.rs. */
    public static class SearchNodesResponse {
        @JsonProperty("nodes") public List<GraphNode> nodes;
        @JsonProperty("edges") public List<GraphEdge> edges;
        @JsonProperty("total_matches") public Integer totalMatches;
    }

    public static class NeighborhoodResponse {
        @JsonProperty("center") public Entity center;
        @JsonProperty("nodes") public List<GraphNode> nodes;
        @JsonProperty("edges") public List<GraphEdge> edges;
        @JsonProperty("depth") public int depth;
    }

    // ── OODA-38: Added missing graph models ──────────────────────────

    /** Graph statistics response. */
    public static class GraphStatsResponse {
        @JsonProperty("node_count") public int nodeCount;
        @JsonProperty("edge_count") public int edgeCount;
        @JsonProperty("entity_count") public int entityCount;
        @JsonProperty("relationship_count") public int relationshipCount;
    }

    /** Label search response. */
    public static class LabelSearchResponse {
        @JsonProperty("labels") public List<LabelMatch> labels;
        @JsonProperty("total") public int total;
    }

    /** Label match info. */
    public static class LabelMatch {
        @JsonProperty("label") public String label;
        @JsonProperty("count") public int count;
    }

    /** Popular labels response. */
    public static class PopularLabelsResponse {
        @JsonProperty("labels") public List<LabelMatch> labels;
    }

    /** Batch degrees response. */
    public static class BatchDegreesResponse {
        @JsonProperty("degrees") public Map<String, Integer> degrees;
    }

    /** Entity types response. */
    public static class EntityTypesResponse {
        @JsonProperty("types") public List<String> types;
        @JsonProperty("total") public int total;
    }

    /** Relationship types response. */
    public static class RelationshipTypesResponse {
        @JsonProperty("types") public List<String> types;
        @JsonProperty("total") public int total;
    }

    /** Relationship detail response. */
    public static class RelationshipDetailResponse {
        @JsonProperty("relationship") public Relationship relationship;
        @JsonProperty("source") public Entity source;
        @JsonProperty("target") public Entity target;
    }
}

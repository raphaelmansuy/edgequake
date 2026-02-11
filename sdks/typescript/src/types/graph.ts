/**
 * Graph, Entity, and Relationship types.
 *
 * @module types/graph
 * @see edgequake/crates/edgequake-api/src/handlers/graph_types.rs
 * @see edgequake/crates/edgequake-api/src/handlers/entities_types.rs
 * @see edgequake/crates/edgequake-api/src/handlers/relationships_types.rs
 */

import type { Timestamp } from "./common.js";

// ── Graph ─────────────────────────────────────────────────────

export interface GraphQuery {
  limit?: number;
  labels?: string[];
  search?: string;
}

export interface GraphNode {
  id: string;
  label: string;
  /** Node type (entity type). */
  node_type?: string;
  /** Node description. */
  description?: string;
  properties?: Record<string, unknown>;
  /** Number of connections. */
  degree?: number;
}

export interface GraphEdge {
  source: string;
  target: string;
  /** Edge type (relationship label). */
  edge_type: string;
  /** Edge weight. */
  weight?: number;
  properties?: Record<string, unknown>;
}

export interface GraphResponse {
  nodes: GraphNode[];
  edges: GraphEdge[];
  /** Whether the graph was truncated. */
  is_truncated?: boolean;
  total_nodes: number;
  total_edges: number;
}

export type GraphStreamEvent =
  | { type: "node"; node: GraphNode }
  | { type: "edge"; edge: GraphEdge }
  | { type: "done"; total_nodes: number; total_edges: number };

export interface SearchNodesResponse {
  nodes: GraphNode[];
  /** Edges connecting the returned nodes. */
  edges?: GraphEdge[];
  /** Total matches in database (before limit). */
  total_matches?: number;
  /** Whether results were truncated. */
  is_truncated?: boolean;
  /** @deprecated Use total_matches instead. */
  total?: number;
}

export interface SearchLabelsResponse {
  labels: string[];
}

export interface PopularLabelsResponse {
  labels: Array<{
    label: string;
    entity_type: string;
    degree: number;
    description: string;
  }>;
  total_entities?: number;
}

export interface DegreesBatchResponse {
  degrees: Record<string, number>;
}

export interface SearchParams {
  limit?: number;
  offset?: number;
}

// ── Entities ──────────────────────────────────────────────────

export interface ListEntitiesQuery {
  page?: number;
  per_page?: number;
  search?: string;
  label?: string;
}

export interface EntitiesListResponse {
  entities: EntityInfo[];
  total: number;
  page: number;
  per_page: number;
}

export interface EntityInfo {
  name: string;
  label: string;
  description?: string;
  source_count?: number;
  created_at?: Timestamp;
}

export interface EntityResponse {
  name: string;
  label: string;
  description?: string;
  properties?: Record<string, unknown>;
  source_documents?: string[];
}

export interface CreateEntityRequest {
  /** Entity name (will be normalized to UPPERCASE). */
  entity_name: string;
  /** Entity type (e.g., PERSON, ORGANIZATION, TECHNOLOGY). */
  entity_type: string;
  /** Entity description. */
  description: string;
  /** Source document ID (use "manual_entry" for manual entries). */
  source_id: string;
  /** Additional metadata. */
  metadata?: Record<string, unknown>;
}

export interface UpdateEntityRequest {
  description?: string;
  properties?: Record<string, unknown>;
}

export interface MergeEntitiesRequest {
  source_entity: string;
  target_entity: string;
  strategy?: "keep_target" | "keep_source" | "merge";
}

export interface MergeEntitiesResponse {
  merged_entity: string;
  removed_entity: string;
  relationships_updated: number;
}

export interface NeighborhoodResponse {
  center: EntityResponse;
  neighbors: Array<{
    entity: EntityResponse;
    relationship: string;
    direction: "incoming" | "outgoing";
  }>;
  depth: number;
}

// ── Relationships ─────────────────────────────────────────────

export interface ListRelationshipsQuery {
  page?: number;
  per_page?: number;
  source?: string;
  target?: string;
  label?: string;
}

export interface RelationshipsListResponse {
  relationships: RelationshipInfo[];
  total: number;
  page: number;
  per_page: number;
}

export interface RelationshipInfo {
  id: string;
  source: string;
  target: string;
  label: string;
  weight?: number;
  description?: string;
  created_at?: Timestamp;
}

export interface RelationshipResponse extends RelationshipInfo {
  properties?: Record<string, unknown>;
  source_documents?: string[];
}

export interface CreateRelationshipRequest {
  source: string;
  target: string;
  label: string;
  weight?: number;
  description?: string;
  properties?: Record<string, unknown>;
}

export interface UpdateRelationshipRequest {
  weight?: number;
  description?: string;
  properties?: Record<string, unknown>;
}

// ── Query Helpers ─────────────────────────────────────────────

export interface SearchNodesQuery {
  q?: string;
  limit?: number;
}

export interface SearchLabelsQuery {
  q?: string;
  limit?: number;
}

export interface DegreeBatchRequest {
  node_ids: string[];
}

export interface DegreeBatchResponse {
  degrees: Record<string, number>;
}

// ── Type Aliases for resource usage ───────────────────────────

/** Entity detail (alias for EntityResponse). */
export type EntityDetail = EntityResponse;

/** Entity neighborhood (alias for NeighborhoodResponse). */
export type EntityNeighborhood = NeighborhoodResponse;

/** Relationship detail (alias for RelationshipResponse). */
export type RelationshipDetail = RelationshipResponse;

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
  properties?: Record<string, unknown>;
  degree?: number;
}

export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  label: string;
  weight?: number;
  properties?: Record<string, unknown>;
}

export interface GraphResponse {
  nodes: GraphNode[];
  edges: GraphEdge[];
  total_nodes: number;
  total_edges: number;
}

export type GraphStreamEvent =
  | { type: "node"; node: GraphNode }
  | { type: "edge"; edge: GraphEdge }
  | { type: "done"; total_nodes: number; total_edges: number };

export interface SearchNodesResponse {
  nodes: GraphNode[];
  total: number;
}

export interface SearchLabelsResponse {
  labels: Array<{ label: string; count: number }>;
}

export interface PopularLabelsResponse {
  labels: Array<{ label: string; count: number }>;
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
  name: string;
  label: string;
  description?: string;
  properties?: Record<string, unknown>;
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

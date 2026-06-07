/**
 * Domain API module — split from edgequake.ts (SPEC-017 UI-DRY-001).
 */

import { api, streamClient } from "../client";

import type {
  Entity,
  GraphEdge,
  GraphNode,
  KnowledgeGraph,
  MergeEntitiesRequest,
  MergeEntitiesResponse,
  PaginatedResponse,
  PaginationParams,
  Relationship,
} from "@/types";

/**
 * Options for fetching the knowledge graph.
 * Supports server-side filtering for 100k+ node graphs.
 */
export interface GetGraphOptions {
  /** Maximum number of nodes to return (default: 500) */
  limit?: number;
  /** Explicit max_nodes parameter (takes precedence over limit) */
  maxNodes?: number;
  /** Maximum traversal depth from start_node (default: 2) */
  depth?: number;
  /** Focus on a specific node and its neighborhood */
  startNode?: string;
  /** Filter by entity types */
  entity_types?: string[];
  /** Include orphan nodes with no connections */
  include_orphans?: boolean;
}

export async function getGraph(
  options?: GetGraphOptions,
): Promise<KnowledgeGraph> {
  const searchParams = new URLSearchParams();

  // Support both limit and maxNodes (maxNodes takes precedence)
  const nodeLimit = options?.maxNodes ?? options?.limit;
  if (nodeLimit) searchParams.set("max_nodes", String(nodeLimit));

  if (options?.depth) searchParams.set("depth", String(options.depth));
  if (options?.startNode) searchParams.set("start_node", options.startNode);
  if (options?.entity_types)
    searchParams.set("entity_types", options.entity_types.join(","));
  if (options?.include_orphans !== undefined) {
    searchParams.set("include_orphans", String(options.include_orphans));
  }

  const query = searchParams.toString();
  const graph = await api.get<KnowledgeGraph>(`/graph${query ? `?${query}` : ""}`);

  // WHY: Normalize edge field names.  Older backend versions returned `edge_type`
  // instead of `relationship_type`.  Map the legacy field so label rendering
  // always works regardless of the API version in use.
  if (graph.edges) {
    graph.edges = graph.edges.map((e) => ({
      ...e,
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      relationship_type: e.relationship_type || (e as any).edge_type || "",
    }));
  }

  return graph;
}

export async function getGraphLabels(): Promise<{
  entity_types: string[];
  relationship_types: string[];
}> {
  return api.get<{ entity_types: string[]; relationship_types: string[] }>(
    "/graph/labels",
  );
}

export async function getGraphStats(): Promise<{
  node_count: number;
  edge_count: number;
  entity_type_counts: Record<string, number>;
  relationship_type_counts: Record<string, number>;
}> {
  return api.get<{
    node_count: number;
    edge_count: number;
    entity_type_counts: Record<string, number>;
    relationship_type_counts: Record<string, number>;
  }>("/graph/stats");
}

/**
 * Search for labels/entities by query string.
 * Used for autocomplete in label search.
 */
export async function searchLabels(
  query: string,
  limit = 20,
): Promise<{ labels: string[] }> {
  return api.get<{ labels: string[] }>(
    `/graph/labels/search?q=${encodeURIComponent(query)}&limit=${limit}`,
  );
}

/**
 * Parameters for full node search.
 */
export interface SearchNodesParams {
  q: string;
  limit?: number;
  includeNeighbors?: boolean;
  neighborDepth?: number;
  entityType?: string;
}

/**
 * Response from full node search.
 */
export interface SearchNodesResponse {
  nodes: GraphNode[];
  edges: GraphEdge[];
  total_matches: number;
  is_truncated: boolean;
}

/**
 * Search for nodes with full data (label and description search).
 * Returns matching nodes with their degrees and connecting edges.
 * Supports optional neighbor inclusion for graph visualization.
 */
export async function searchNodes(
  params: SearchNodesParams,
): Promise<SearchNodesResponse> {
  const urlParams = new URLSearchParams();
  urlParams.set("q", params.q);
  if (params.limit) urlParams.set("limit", String(params.limit));
  if (params.includeNeighbors !== undefined)
    urlParams.set("include_neighbors", String(params.includeNeighbors));
  if (params.neighborDepth !== undefined)
    urlParams.set("neighbor_depth", String(params.neighborDepth));
  if (params.entityType) urlParams.set("entity_type", params.entityType);

  return api.get<SearchNodesResponse>(`/graph/nodes/search?${urlParams}`);
}

/**
 * Popular label with metadata.
 */
export interface PopularLabel {
  label: string;
  entity_type: string;
  degree: number;
  description: string;
}

/**
 * Get popular entities/labels sorted by connection count.
 * Useful for quick access to high-connectivity nodes.
 */
export async function getPopularLabels(options?: {
  limit?: number;
  minDegree?: number;
  entityType?: string;
}): Promise<{ labels: PopularLabel[]; total_entities: number }> {
  const params = new URLSearchParams();
  if (options?.limit) params.set("limit", String(options.limit));
  if (options?.minDegree) params.set("min_degree", String(options.minDegree));
  if (options?.entityType) params.set("entity_type", options.entityType);
  const query = params.toString();
  return api.get(`/graph/labels/popular${query ? `?${query}` : ""}`);
}

/**
 * Metadata sent at the start of graph streaming.
 */
export interface GraphStreamMetadata {
  total_nodes: number;
  total_edges: number;
  nodes_to_stream: number;
  edges_to_stream: number;
}

/**
 * Statistics sent at the end of graph streaming.
 */
export interface GraphStreamStats {
  nodes_count: number;
  edges_count: number;
  duration_ms: number;
}

/**
 * SSE events emitted during graph streaming.
 * Events are sent in order: metadata → nodes (batches) → edges → done
 */
export type GraphStreamEvent =
  | {
      type: "metadata";
      total_nodes: number;
      total_edges: number;
      nodes_to_stream: number;
      edges_to_stream: number;
    }
  | { type: "nodes"; batch: number; total_batches: number; nodes: GraphNode[] }
  | { type: "edges"; edges: GraphEdge[] }
  | {
      type: "done";
      nodes_count: number;
      edges_count: number;
      duration_ms: number;
    }
  | { type: "error"; message: string };

/**
 * Options for streaming graph fetch.
 */
export interface GetGraphStreamOptions {
  /** Maximum nodes to stream (default: 200) */
  maxNodes?: number;
  /** Nodes per batch (default: 50) */
  batchSize?: number;
  /** Focus on specific node neighborhood */
  startNode?: string;
}

/**
 * Stream graph data progressively via SSE.
 *
 * This function yields events as they arrive from the server:
 * 1. `metadata` - Initial graph statistics
 * 2. `nodes` - Multiple batches of nodes (batch_size per event)
 * 3. `edges` - Edges between streamed nodes
 * 4. `done` - Completion summary with timing
 *
 * @example
 * ```typescript
 * for await (const event of graphStream({ maxNodes: 200 })) {
 *   switch (event.type) {
 *     case 'metadata':
 *       console.log(`Streaming ${event.nodes_to_stream} nodes`);
 *       break;
 *     case 'nodes':
 *       console.log(`Batch ${event.batch}/${event.total_batches}`);
 *       addNodesToGraph(event.nodes);
 *       break;
 *     case 'edges':
 *       setEdges(event.edges);
 *       break;
 *     case 'done':
 *       console.log(`Completed in ${event.duration_ms}ms`);
 *       break;
 *   }
 * }
 * ```
 */
export async function* graphStream(
  options?: GetGraphStreamOptions,
): AsyncGenerator<GraphStreamEvent, void, unknown> {
  const searchParams = new URLSearchParams();
  if (options?.maxNodes)
    searchParams.set("max_nodes", String(options.maxNodes));
  if (options?.batchSize)
    searchParams.set("batch_size", String(options.batchSize));
  if (options?.startNode) searchParams.set("start_node", options.startNode);

  const query = searchParams.toString();
  yield* streamClient<GraphStreamEvent>(
    `/graph/stream${query ? `?${query}` : ""}`,
    {
      method: "GET",
    },
  );
}

export async function getEntities(
  params?: PaginationParams & { entity_type?: string; search?: string },
): Promise<PaginatedResponse<Entity>> {
  const searchParams = new URLSearchParams();
  if (params?.page) searchParams.set("page", String(params.page));
  if (params?.page_size)
    searchParams.set("page_size", String(params.page_size));
  if (params?.entity_type) searchParams.set("entity_type", params.entity_type);
  if (params?.search) searchParams.set("search", params.search);

  const query = searchParams.toString();
  return api.get<PaginatedResponse<Entity>>(
    `/graph/entities${query ? `?${query}` : ""}`,
  );
}

export async function getEntity(entityId: string): Promise<Entity> {
  return api.get<Entity>(`/graph/entities/${entityId}`);
}

export async function updateEntity(
  entityId: string,
  data: Partial<Entity>,
): Promise<Entity> {
  return api.put<Entity>(`/graph/entities/${entityId}`, data);
}

export async function deleteEntity(entityId: string): Promise<void> {
  return api.delete<void>(`/graph/entities/${entityId}`);
}

export async function mergeEntities(
  request: MergeEntitiesRequest,
): Promise<MergeEntitiesResponse> {
  return api.post<MergeEntitiesResponse>("/graph/entities/merge", request);
}

export async function getEntityNeighborhood(
  entityId: string,
  depth?: number,
): Promise<{ nodes: GraphNode[]; edges: GraphEdge[] }> {
  const query = depth ? `?depth=${depth}` : "";
  return api.get<{ nodes: GraphNode[]; edges: GraphEdge[] }>(
    `/graph/entities/${entityId}/neighborhood${query}`,
  );
}

export async function getRelationships(
  params?: PaginationParams & { relationship_type?: string },
): Promise<PaginatedResponse<Relationship>> {
  const searchParams = new URLSearchParams();
  if (params?.page) searchParams.set("page", String(params.page));
  if (params?.page_size)
    searchParams.set("page_size", String(params.page_size));
  if (params?.relationship_type)
    searchParams.set("relationship_type", params.relationship_type);

  const query = searchParams.toString();
  return api.get<PaginatedResponse<Relationship>>(
    `/graph/relationships${query ? `?${query}` : ""}`,
  );
}

export async function getRelationship(
  relationshipId: string,
): Promise<Relationship> {
  return api.get<Relationship>(`/graph/relationships/${relationshipId}`);
}

export async function updateRelationship(
  relationshipId: string,
  data: Partial<Relationship>,
): Promise<Relationship> {
  return api.put<Relationship>(`/graph/relationships/${relationshipId}`, data);
}

export async function deleteRelationship(
  relationshipId: string,
): Promise<void> {
  return api.delete<void>(`/graph/relationships/${relationshipId}`);
}

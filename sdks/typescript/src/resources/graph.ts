/**
 * Graph resource — knowledge graph queries, entities, and relationships.
 *
 * @module resources/graph
 * @see edgequake/crates/edgequake-api/src/handlers/graph.rs
 */

import type { HttpTransport } from "../transport/types.js";
import type {
  CreateEntityRequest,
  CreateRelationshipRequest,
  DegreeBatchRequest,
  DegreeBatchResponse,
  EntityDetail,
  EntityNeighborhood,
  GraphNode,
  GraphQuery,
  GraphResponse,
  GraphStreamEvent,
  ListEntitiesQuery,
  ListRelationshipsQuery,
  MergeEntitiesRequest,
  RelationshipDetail,
  SearchLabelsQuery,
  SearchNodesQuery,
  UpdateEntityRequest,
  UpdateRelationshipRequest,
} from "../types/graph.js";
import { Resource } from "./base.js";

/** Entities sub-resource accessed via `client.graph.entities`. */
export class EntitiesResource extends Resource {
  /** List entities with optional filters. */
  async list(query?: ListEntitiesQuery): Promise<EntityDetail[]> {
    const params = new URLSearchParams();
    if (query?.label) params.set("label", query.label);
    if (query?.search) params.set("search", query.search);
    if (query?.page !== undefined) params.set("page", String(query.page));
    if (query?.per_page !== undefined)
      params.set("per_page", String(query.per_page));
    const qs = params.toString();
    const path = qs ? `/api/v1/graph/entities?${qs}` : "/api/v1/graph/entities";
    // WHY: API returns paginated { items: [...], total, page, page_size }
    const raw = await this._get<{
      items: EntityDetail[];
      [key: string]: unknown;
    }>(path);
    return raw.items ?? (raw as unknown as EntityDetail[]);
  }

  /** Create a new entity. */
  async create(request: CreateEntityRequest): Promise<EntityDetail> {
    return this._post("/api/v1/graph/entities", request);
  }

  /** Get entity by name. */
  async get(entityName: string): Promise<EntityDetail> {
    return this._get(
      `/api/v1/graph/entities/${encodeURIComponent(entityName)}`,
    );
  }

  /** Check if an entity exists. */
  async exists(entityName: string): Promise<boolean> {
    // WHY: API expects `entity_name` query param, not `name`
    const resp = await this._get<{ exists: boolean }>(
      `/api/v1/graph/entities/exists?entity_name=${encodeURIComponent(entityName)}`,
    );
    return resp.exists;
  }

  /** Update an entity. */
  async update(
    entityName: string,
    request: UpdateEntityRequest,
  ): Promise<EntityDetail> {
    return this._put(
      `/api/v1/graph/entities/${encodeURIComponent(entityName)}`,
      request,
    );
  }

  /** Delete an entity. */
  async delete(entityName: string): Promise<void> {
    await this._del(`/api/v1/graph/entities/${encodeURIComponent(entityName)}`);
  }

  /** Merge two entities into one. */
  async merge(request: MergeEntitiesRequest): Promise<EntityDetail> {
    return this._post("/api/v1/graph/entities/merge", request);
  }

  /** Get entity neighborhood (connected nodes and edges). */
  async neighborhood(entityName: string): Promise<EntityNeighborhood> {
    return this._get(
      `/api/v1/graph/entities/${encodeURIComponent(entityName)}/neighborhood`,
    );
  }
}

/** Relationships sub-resource accessed via `client.graph.relationships`. */
export class RelationshipsResource extends Resource {
  /** List relationships with optional filters. */
  async list(query?: ListRelationshipsQuery): Promise<RelationshipDetail[]> {
    const params = new URLSearchParams();
    if (query?.source) params.set("source", query.source);
    if (query?.target) params.set("target", query.target);
    if (query?.label) params.set("label", query.label);
    if (query?.page !== undefined) params.set("page", String(query.page));
    if (query?.per_page !== undefined)
      params.set("per_page", String(query.per_page));
    const qs = params.toString();
    const path = qs
      ? `/api/v1/graph/relationships?${qs}`
      : "/api/v1/graph/relationships";
    // WHY: API returns paginated { items: [...], total, page, page_size }
    const raw = await this._get<{
      items: RelationshipDetail[];
      [key: string]: unknown;
    }>(path);
    return raw.items ?? (raw as unknown as RelationshipDetail[]);
  }

  /** Create a new relationship. */
  async create(
    request: CreateRelationshipRequest,
  ): Promise<RelationshipDetail> {
    return this._post("/api/v1/graph/relationships", request);
  }

  /** Get relationship by ID. */
  async get(relationshipId: string): Promise<RelationshipDetail> {
    return this._get(`/api/v1/graph/relationships/${relationshipId}`);
  }

  /** Update a relationship. */
  async update(
    relationshipId: string,
    request: UpdateRelationshipRequest,
  ): Promise<RelationshipDetail> {
    return this._put(`/api/v1/graph/relationships/${relationshipId}`, request);
  }

  /** Delete a relationship. */
  async delete(relationshipId: string): Promise<void> {
    await this._del(`/api/v1/graph/relationships/${relationshipId}`);
  }
}

/** Graph resource with entities and relationships sub-namespaces. */
export class GraphResource extends Resource {
  /** Entities sub-resource. */
  readonly entities: EntitiesResource;

  /** Relationships sub-resource. */
  readonly relationships: RelationshipsResource;

  constructor(transport: HttpTransport) {
    super(transport);
    this.entities = new EntitiesResource(transport);
    this.relationships = new RelationshipsResource(transport);
  }

  /** Get the knowledge graph (nodes and edges). */
  async get(query?: GraphQuery): Promise<GraphResponse> {
    const params = new URLSearchParams();
    if (query?.limit !== undefined) params.set("limit", String(query.limit));
    if (query?.labels?.length) params.set("labels", query.labels.join(","));
    if (query?.search) params.set("search", query.search);
    const qs = params.toString();
    const path = qs ? `/api/v1/graph?${qs}` : "/api/v1/graph";
    return this._get(path);
  }

  /** Stream graph data as server-sent events. */
  stream(
    query?: GraphQuery,
    signal?: AbortSignal,
  ): AsyncIterable<GraphStreamEvent> {
    const params = new URLSearchParams();
    if (query?.limit !== undefined) params.set("limit", String(query.limit));
    if (query?.labels?.length) params.set("labels", query.labels.join(","));
    if (query?.search) params.set("search", query.search);
    const qs = params.toString();
    const path = qs ? `/api/v1/graph/stream?${qs}` : "/api/v1/graph/stream";
    return this._streamSSE<GraphStreamEvent>(path, undefined, signal);
  }

  /** Get a specific node by ID. */
  async getNode(nodeId: string): Promise<GraphNode> {
    return this._get(`/api/v1/graph/nodes/${nodeId}`);
  }

  /** Search nodes by query. */
  async searchNodes(query: SearchNodesQuery): Promise<GraphNode[]> {
    const params = new URLSearchParams();
    if (query.q) params.set("q", query.q);
    if (query.limit !== undefined) params.set("limit", String(query.limit));
    return this._get(`/api/v1/graph/nodes/search?${params}`);
  }

  /** Search graph labels. */
  async searchLabels(query: SearchLabelsQuery): Promise<string[]> {
    const params = new URLSearchParams();
    if (query.q) params.set("q", query.q);
    if (query.limit !== undefined) params.set("limit", String(query.limit));
    return this._get(`/api/v1/graph/labels/search?${params}`);
  }

  /** Get popular labels. */
  async getPopularLabels(): Promise<string[]> {
    return this._get("/api/v1/graph/labels/popular");
  }

  /** Get degrees for multiple nodes in batch. */
  async getDegreesBatch(
    request: DegreeBatchRequest,
  ): Promise<DegreeBatchResponse> {
    return this._post("/api/v1/graph/degrees/batch", request);
  }
}

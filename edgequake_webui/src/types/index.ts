// Graph types
export interface GraphNode {
  id: string;
  label: string;
  node_type: string;
  description?: string;
  degree?: number;
  properties?: Record<string, unknown>;
  created_at?: string;
  updated_at?: string;
}

export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  relationship_type: string;
  weight: number;
  description?: string;
  source_ids: string[];
  properties?: Record<string, unknown>;
  created_at: string;
}

export interface KnowledgeGraph {
  nodes: GraphNode[];
  edges: GraphEdge[];
  metadata: {
    node_count: number;
    edge_count: number;
    entity_types: string[];
    relationship_types: string[];
  };
}

// Document types
export interface Document {
  id: string;
  title?: string | null;
  content?: string;
  source_type?: "file" | "text" | "url";
  status?: "pending" | "processing" | "completed" | "failed";
  error_message?: string;
  file_name?: string;
  file_size?: number;
  mime_type?: string;
  chunk_count?: number;
  entity_count?: number;
  created_at?: string;
  updated_at?: string;
  processed_at?: string;
}

export interface DocumentChunk {
  id: string;
  document_id: string;
  content: string;
  chunk_index: number;
  tokens: number;
  embedding_id?: string;
}

export interface UploadDocumentRequest {
  content: string;
  title?: string;
  source_type?: "text" | "file" | "url";
  metadata?: Record<string, unknown>;
}

export interface UploadDocumentResponse {
  document_id: string;
  status: string;
  message: string;
}

// Query types
export type QueryMode = "local" | "global" | "hybrid" | "naive";

export interface QueryRequest {
  query: string;
  mode: QueryMode;
  top_k?: number;
  max_tokens?: number;
  temperature?: number;
  stream?: boolean;
  only_context?: boolean;
}

export interface QueryContext {
  chunks: Array<{
    content: string;
    document_id: string;
    score: number;
  }>;
  entities: Array<{
    id: string;
    label: string;
    relevance: number;
  }>;
  relationships: Array<{
    source: string;
    target: string;
    type: string;
    relevance: number;
  }>;
}

export interface QueryResponse {
  answer: string;
  context: QueryContext;
  mode: QueryMode;
  tokens_used: number;
  duration_ms: number;
}

export interface QueryStreamChunk {
  type: "token" | "context" | "done" | "error";
  content?: string;
  context?: QueryContext;
  error?: string;
  tokens_used?: number;
  duration_ms?: number;
}

// Auth types
export interface LoginRequest {
  username: string;
  password: string;
}

export interface LoginResponse {
  access_token: string;
  refresh_token: string;
  token_type: string;
  expires_in: number;
  user: {
    id: string;
    username: string;
    email?: string;
    roles: string[];
  };
}

export interface AuthState {
  isAuthenticated: boolean;
  user: LoginResponse["user"] | null;
  accessToken: string | null;
  refreshToken: string | null;
  expiresAt: number | null;
}

// Tenant types
export interface Tenant {
  id: string;
  name: string;
  description?: string;
  created_at: string;
}

export interface Workspace {
  id: string;
  tenant_id: string;
  name: string;
  description?: string;
  document_count: number;
  entity_count: number;
  created_at: string;
}

// Task/Pipeline types
export interface PipelineTask {
  id: string;
  document_id: string;
  status: "queued" | "running" | "completed" | "failed";
  stage: "parsing" | "chunking" | "extraction" | "embedding" | "indexing";
  progress: number;
  error_message?: string;
  started_at?: string;
  completed_at?: string;
}

export interface PipelineStatus {
  is_busy: boolean;
  running_tasks: number;
  queued_tasks: number;
  completed_tasks: number;
  failed_tasks: number;
  tasks: PipelineTask[];
  // Extended fields for pipeline dialog
  job_name?: string;
  start_time?: string;
  progress?: number;
  current?: number;
  total?: number;
  messages?: string[];
}

// Health types
export interface HealthResponse {
  status: "healthy" | "degraded" | "unhealthy";
  version: string;
  uptime_seconds: number;
  components: {
    database: "up" | "down";
    llm_provider: "up" | "down";
    storage: "up" | "down";
  };
}

// Entity types
export interface Entity {
  id: string;
  label: string;
  entity_type: string;
  description?: string;
  aliases: string[];
  source_ids: string[];
  properties: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface MergeEntitiesRequest {
  source_ids: string[];
  target_label: string;
  target_type?: string;
}

export interface MergeEntitiesResponse {
  merged_entity: Entity;
  merged_count: number;
}

// Relationship types
export interface Relationship {
  id: string;
  source_entity_id: string;
  target_entity_id: string;
  relationship_type: string;
  weight: number;
  description?: string;
  source_ids: string[];
  properties: Record<string, unknown>;
  created_at: string;
}

// Settings types
export interface GraphSettings {
  showLabels: boolean;
  showEdgeLabels: boolean;
  nodeSize: "small" | "medium" | "large";
  edgeThickness: "thin" | "medium" | "thick";
  layout: "force" | "circular" | "random";
  colorBy: "type" | "community" | "degree";
}

export interface QuerySettings {
  mode: QueryMode;
  topK: number;
  maxTokens: number;
  temperature: number;
  stream: boolean;
}

export interface AppSettings {
  theme: "light" | "dark" | "system";
  language: "en" | "zh" | "ja" | "ko";
  graphSettings: GraphSettings;
  querySettings: QuerySettings;
}

// API error types
export interface ApiError {
  message: string;
  code?: string;
  details?: Record<string, unknown>;
  status: number;
}

// Pagination types
export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  page_size: number;
  has_more: boolean;
}

export interface PaginationParams {
  page?: number;
  page_size?: number;
  sort_by?: string;
  sort_order?: "asc" | "desc";
}

// Query history
export interface QueryHistoryItem {
  id: string;
  query: string;
  mode: QueryMode;
  response?: string;
  timestamp: string;
  isFavorite: boolean;
}

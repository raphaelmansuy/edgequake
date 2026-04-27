package edgequake

// UUID is a convenience alias.
type UUID = string

// HealthResponse from GET /health.
type HealthResponse struct {
	Status      string          `json:"status"`
	Version     string          `json:"version,omitempty"`
	StorageMode string          `json:"storage_mode,omitempty"`
	WorkspaceID string          `json:"workspace_id,omitempty"`
	Components  map[string]bool `json:"components,omitempty"`
	LLMProvider string          `json:"llm_provider_name,omitempty"`
}

type Document struct {
	ID          UUID   `json:"id"`
	FileName    string `json:"file_name,omitempty"`
	Title       string `json:"title,omitempty"`
	Status      string `json:"status,omitempty"`
	FileSize    *int64 `json:"file_size,omitempty"`
	MimeType    string `json:"mime_type,omitempty"`
	EntityCount *int   `json:"entity_count,omitempty"`
	ChunkCount  *int   `json:"chunk_count,omitempty"`
	CreatedAt   string `json:"created_at,omitempty"`
	UpdatedAt   string `json:"updated_at,omitempty"`
}

type UploadResponse struct {
	ID                string `json:"document_id"`
	Status            string `json:"status,omitempty"`
	TrackID           string `json:"track_id,omitempty"`
	Message           string `json:"message,omitempty"`
	ChunkCount        *int   `json:"chunk_count,omitempty"`
	EntityCount       *int   `json:"entity_count,omitempty"`
	RelationshipCount *int   `json:"relationship_count,omitempty"`
}

type ListDocumentsResponse struct {
	Documents  []Document      `json:"documents"`
	Pagination *PaginationInfo `json:"pagination,omitempty"`
}

type PaginationInfo struct {
	Page       int `json:"page"`
	PerPage    int `json:"per_page"`
	Total      int `json:"total"`
	TotalPages int `json:"total_pages"`
}

type TrackStatus struct {
	TrackID    string   `json:"track_id"`
	Status     string   `json:"status"`
	Progress   *float64 `json:"progress,omitempty"`
	Message    string   `json:"message,omitempty"`
	DocumentID string   `json:"document_id,omitempty"`
}

type ScanRequest struct {
	Path       string   `json:"path"`
	Recursive  *bool    `json:"recursive,omitempty"`
	Extensions []string `json:"extensions,omitempty"`
}

type ScanResponse struct {
	FilesFound   int `json:"files_found"`
	FilesQueued  int `json:"files_queued"`
	FilesSkipped int `json:"files_skipped"`
}

type DeletionImpact struct {
	EntityCount       int `json:"entity_count"`
	RelationshipCount int `json:"relationship_count"`
	ChunkCount        int `json:"chunk_count"`
}

type GraphNode struct {
	ID          string                 `json:"id"`
	Label       string                 `json:"label"`
	NodeType    string                 `json:"node_type,omitempty"`
	Description string                 `json:"description,omitempty"`
	Properties  map[string]interface{} `json:"properties,omitempty"`
	Degree      *int                   `json:"degree,omitempty"`
}

type GraphEdge struct {
	Source     string                 `json:"source"`
	Target     string                 `json:"target"`
	EdgeType   string                 `json:"edge_type,omitempty"`
	Weight     *float64               `json:"weight,omitempty"`
	Properties map[string]interface{} `json:"properties,omitempty"`
}

type GraphResponse struct {
	Nodes      []GraphNode `json:"nodes"`
	Edges      []GraphEdge `json:"edges"`
	TotalNodes *int        `json:"total_nodes,omitempty"`
	TotalEdges *int        `json:"total_edges,omitempty"`
}

type SearchNodesResponse struct {
	Nodes        []GraphNode `json:"nodes"`
	Edges        []GraphEdge `json:"edges"`
	TotalMatches *int        `json:"total_matches,omitempty"`
}

type Entity struct {
	ID          string                 `json:"id"`
	EntityName  string                 `json:"entity_name"`
	Name        string                 `json:"name,omitempty"`
	EntityType  string                 `json:"entity_type,omitempty"`
	Description string                 `json:"description,omitempty"`
	SourceID    string                 `json:"source_id,omitempty"`
	Properties  map[string]interface{} `json:"properties,omitempty"`
	Degree      *int                   `json:"degree,omitempty"`
	CreatedAt   string                 `json:"created_at,omitempty"`
	UpdatedAt   string                 `json:"updated_at,omitempty"`
	Metadata    interface{}            `json:"metadata,omitempty"`
}

type CreateEntityParams struct {
	EntityName  string      `json:"entity_name"`
	EntityType  string      `json:"entity_type"`
	Description string      `json:"description"`
	SourceID    string      `json:"source_id"`
	Metadata    interface{} `json:"metadata,omitempty"`
}

// CreateEntityResponse is the response from POST /api/v1/graph/entities.
type CreateEntityResponse struct {
	Status  string  `json:"status"`
	Message string  `json:"message"`
	Entity  *Entity `json:"entity"`
}

type MergeEntitiesParams struct {
	SourceEntity string `json:"source_entity"`
	TargetEntity string `json:"target_entity"`
}

type MergeResponse struct {
	MergedEntity *Entity `json:"merged_entity,omitempty"`
	MergedCount  int     `json:"merged_count"`
	Message      string  `json:"message,omitempty"`
}

// EntityDetailResponse is the response from GET /api/v1/graph/entities/{id}.
// It wraps the entity with related relationships and statistics.
type EntityDetailResponse struct {
	Entity        *Entity              `json:"entity"`
	Relationships *EntityRelationships `json:"relationships,omitempty"`
	Statistics    *EntityStatistics    `json:"statistics,omitempty"`
}

type EntityRelationships struct {
	Outgoing []Relationship `json:"outgoing"`
	Incoming []Relationship `json:"incoming"`
}

type EntityStatistics struct {
	TotalRelationships int `json:"total_relationships"`
	OutgoingCount      int `json:"outgoing_count"`
	IncomingCount      int `json:"incoming_count"`
	DocumentReferences int `json:"document_references"`
}

type NeighborhoodResponse struct {
	Center *Entity     `json:"center,omitempty"`
	Nodes  []GraphNode `json:"nodes"`
	Edges  []GraphEdge `json:"edges"`
	Depth  int         `json:"depth"`
}

type EntityExistsResponse struct {
	Exists     bool    `json:"exists"`
	EntityID   *string `json:"entity_id,omitempty"`
	EntityType *string `json:"entity_type,omitempty"`
	Degree     *int    `json:"degree,omitempty"`
}

type Relationship struct {
	ID               string                 `json:"id,omitempty"`
	Source           string                 `json:"source"`
	Target           string                 `json:"target"`
	RelationshipType string                 `json:"relationship_type,omitempty"`
	Weight           *float64               `json:"weight,omitempty"`
	Description      string                 `json:"description,omitempty"`
	Properties       map[string]interface{} `json:"properties,omitempty"`
}

type CreateRelationshipParams struct {
	Source           string   `json:"source"`
	Target           string   `json:"target"`
	RelationshipType string   `json:"relationship_type"`
	Weight           *float64 `json:"weight,omitempty"`
	Description      string   `json:"description,omitempty"`
}

type QueryRequest struct {
	Query           string `json:"query"`
	Mode            string `json:"mode,omitempty"`
	TopK            *int   `json:"top_k,omitempty"`
	Stream          *bool  `json:"stream,omitempty"`
	OnlyNeedContext *bool  `json:"only_need_context,omitempty"`
	SystemPrompt    string `json:"system_prompt,omitempty"`
}

type QueryResponse struct {
	Answer  string            `json:"answer,omitempty"`
	Sources []SourceReference `json:"sources"`
	Mode    string            `json:"mode,omitempty"`
}

type SourceReference struct {
	DocumentID string                 `json:"document_id,omitempty"`
	ChunkID    string                 `json:"chunk_id,omitempty"`
	Content    string                 `json:"content,omitempty"`
	Score      *float64               `json:"score,omitempty"`
	Metadata   map[string]interface{} `json:"metadata,omitempty"`
}

// ChatMessage represents a message in a conversation (used for display/history).
type ChatMessage struct {
	Role    string `json:"role"`
	Content string `json:"content"`
}

// ChatCompletionRequest is the request body for POST /api/v1/chat/completions.
// WHY: EdgeQuake uses `message` (singular string), not `messages` (array).
type ChatCompletionRequest struct {
	Message        string   `json:"message"`
	Stream         *bool    `json:"stream,omitempty"`
	Mode           string   `json:"mode,omitempty"`
	ConversationID string   `json:"conversation_id,omitempty"`
	MaxTokens      *int     `json:"max_tokens,omitempty"`
	Temperature    *float64 `json:"temperature,omitempty"`
	TopK           *int     `json:"top_k,omitempty"`
	ParentID       string   `json:"parent_id,omitempty"`
	Provider       string   `json:"provider,omitempty"`
	Model          string   `json:"model,omitempty"`
	SystemPrompt   string   `json:"system_prompt,omitempty"`
}

// ChatCompletionResponse is the response from POST /api/v1/chat/completions.
// WHY: EdgeQuake returns conversation-threaded response with RAG sources,
// not OpenAI-style choices array.
type ChatCompletionResponse struct {
	ConversationID     string            `json:"conversation_id"`
	UserMessageID      string            `json:"user_message_id"`
	AssistantMessageID string            `json:"assistant_message_id"`
	Content            string            `json:"content"`
	Mode               string            `json:"mode"`
	Sources            []SourceReference `json:"sources"`
}

type LoginParams struct {
	Username string `json:"username"`
	Password string `json:"password"`
}

type TokenResponse struct {
	AccessToken  string `json:"access_token"`
	RefreshToken string `json:"refresh_token,omitempty"`
	TokenType    string `json:"token_type,omitempty"`
	ExpiresIn    *int   `json:"expires_in,omitempty"`
}

type RefreshParams struct {
	RefreshToken string `json:"refresh_token"`
}

type UserInfo struct {
	ID       UUID   `json:"id"`
	Username string `json:"username,omitempty"`
	Email    string `json:"email,omitempty"`
	Role     string `json:"role,omitempty"`
}

type CreateUserParams struct {
	Username string `json:"username"`
	Email    string `json:"email"`
	Password string `json:"password"`
	Role     string `json:"role,omitempty"`
}

type APIKeyResponse struct {
	ID        UUID   `json:"id"`
	Key       string `json:"key"`
	Name      string `json:"name,omitempty"`
	CreatedAt string `json:"created_at,omitempty"`
}

type APIKeyInfo struct {
	ID        UUID   `json:"id"`
	Name      string `json:"name,omitempty"`
	CreatedAt string `json:"created_at,omitempty"`
}

type CreateTenantParams struct {
	Name        string `json:"name"`
	Slug        string `json:"slug,omitempty"`
	Description string `json:"description,omitempty"`
	Plan        string `json:"plan,omitempty"`
	// Default LLM configuration for new workspaces.
	DefaultLLMModel    string `json:"default_llm_model,omitempty"`
	DefaultLLMProvider string `json:"default_llm_provider,omitempty"`
	// Default embedding configuration for new workspaces.
	DefaultEmbeddingModel     string `json:"default_embedding_model,omitempty"`
	DefaultEmbeddingProvider  string `json:"default_embedding_provider,omitempty"`
	DefaultEmbeddingDimension int    `json:"default_embedding_dimension,omitempty"`
	// Default vision LLM for PDF image extraction (SPEC-041). Workspaces inherit this.
	DefaultVisionLLMModel    string `json:"default_vision_llm_model,omitempty"`
	DefaultVisionLLMProvider string `json:"default_vision_llm_provider,omitempty"`
}

type TenantInfo struct {
	ID                        UUID   `json:"id"`
	Name                      string `json:"name"`
	Slug                      string `json:"slug,omitempty"`
	Plan                      string `json:"plan,omitempty"`
	IsActive                  bool   `json:"is_active,omitempty"`
	MaxWorkspaces             int    `json:"max_workspaces,omitempty"`
	DefaultLLMModel           string `json:"default_llm_model,omitempty"`
	DefaultLLMProvider        string `json:"default_llm_provider,omitempty"`
	DefaultEmbeddingModel     string `json:"default_embedding_model,omitempty"`
	DefaultEmbeddingProvider  string `json:"default_embedding_provider,omitempty"`
	DefaultEmbeddingDimension int    `json:"default_embedding_dimension,omitempty"`
	// Vision LLM defaults (SPEC-041) – only present when configured.
	DefaultVisionLLMModel    string `json:"default_vision_llm_model,omitempty"`
	DefaultVisionLLMProvider string `json:"default_vision_llm_provider,omitempty"`
	CreatedAt                string `json:"created_at,omitempty"`
	UpdatedAt                string `json:"updated_at,omitempty"`
}

type CreateConversationParams struct {
	Title    string `json:"title,omitempty"`
	FolderID string `json:"folder_id,omitempty"`
}

type ConversationInfo struct {
	ID           UUID   `json:"id"`
	Title        string `json:"title,omitempty"`
	FolderID     string `json:"folder_id,omitempty"`
	MessageCount int    `json:"message_count"`
	IsPinned     bool   `json:"is_pinned"`
	CreatedAt    string `json:"created_at,omitempty"`
	UpdatedAt    string `json:"updated_at,omitempty"`
}

type ConversationDetail struct {
	ID        UUID      `json:"id"`
	Title     string    `json:"title,omitempty"`
	Messages  []Message `json:"messages"`
	CreatedAt string    `json:"created_at,omitempty"`
}

type Message struct {
	ID        UUID   `json:"id"`
	Role      string `json:"role"`
	Content   string `json:"content"`
	CreatedAt string `json:"created_at,omitempty"`
}

type CreateMessageParams struct {
	Role    string `json:"role"`
	Content string `json:"content"`
}

type ShareLink struct {
	ShareID   string `json:"share_id"`
	URL       string `json:"url,omitempty"`
	CreatedAt string `json:"created_at,omitempty"`
	ExpiresAt string `json:"expires_at,omitempty"`
}

type BulkDeleteResponse struct {
	Affected int `json:"affected"`
}

type FolderInfo struct {
	ID                UUID   `json:"id"`
	Name              string `json:"name"`
	ParentID          string `json:"parent_id,omitempty"`
	ConversationCount int    `json:"conversation_count"`
}

type CreateFolderParams struct {
	Name     string `json:"name"`
	ParentID string `json:"parent_id,omitempty"`
}

type TaskProgress struct {
	CurrentStep     string `json:"current_step,omitempty"`
	PercentComplete int    `json:"percent_complete"`
	TotalSteps      int    `json:"total_steps"`
}

type TaskResult struct {
	DocumentID        string `json:"document_id,omitempty"`
	ChunkCount        int    `json:"chunk_count,omitempty"`
	EntityCount       int    `json:"entity_count,omitempty"`
	RelationshipCount int    `json:"relationship_count,omitempty"`
}

type TaskInfo struct {
	TrackID      string        `json:"track_id"`
	TenantID     string        `json:"tenant_id,omitempty"`
	WorkspaceID  string        `json:"workspace_id,omitempty"`
	TaskType     string        `json:"task_type,omitempty"`
	Status       string        `json:"status"`
	CreatedAt    string        `json:"created_at,omitempty"`
	UpdatedAt    string        `json:"updated_at,omitempty"`
	StartedAt    string        `json:"started_at,omitempty"`
	CompletedAt  string        `json:"completed_at,omitempty"`
	ErrorMessage *string       `json:"error_message"`
	RetryCount   int           `json:"retry_count"`
	MaxRetries   int           `json:"max_retries"`
	Progress     *TaskProgress `json:"progress,omitempty"`
	Result       *TaskResult   `json:"result,omitempty"`
}

type TaskListResponse struct {
	Tasks []TaskInfo `json:"tasks"`
	Total int        `json:"total"`
}

type PipelineStatus struct {
	Status         string `json:"status"`
	ActiveTasks    int    `json:"active_tasks"`
	QueuedTasks    int    `json:"queued_tasks"`
	CompletedTasks int    `json:"completed_tasks"`
	FailedTasks    int    `json:"failed_tasks"`
}

type QueueMetrics struct {
	QueueDepth        int      `json:"queue_depth"`
	Processing        int      `json:"processing"`
	CompletedLastHour int      `json:"completed_last_hour"`
	FailedLastHour    int      `json:"failed_last_hour"`
	AvgProcessingMs   *float64 `json:"avg_processing_time_ms,omitempty"`
}

type CostSummary struct {
	TotalCostUSD      float64 `json:"total_cost_usd"`
	TotalTokens       int64   `json:"total_tokens"`
	TotalInputTokens  int64   `json:"total_input_tokens"`
	TotalOutputTokens int64   `json:"total_output_tokens"`
	DocumentCount     int     `json:"document_count"`
	QueryCount        int     `json:"query_count"`
}

type CostEntry struct {
	Date     string  `json:"date"`
	CostUSD  float64 `json:"cost_usd"`
	Tokens   int64   `json:"tokens"`
	Requests int     `json:"requests"`
}

type BudgetInfo struct {
	MonthlyBudgetUSD *float64 `json:"monthly_budget_usd,omitempty"`
	CurrentSpendUSD  float64  `json:"current_spend_usd"`
	RemainingUSD     *float64 `json:"remaining_usd,omitempty"`
}

type ChunkDetail struct {
	ID         UUID   `json:"id"`
	DocumentID string `json:"document_id,omitempty"`
	Content    string `json:"content,omitempty"`
	ChunkIndex *int   `json:"chunk_index,omitempty"`
	TokenCount *int   `json:"token_count,omitempty"`
}

type ProvenanceRecord struct {
	EntityID         string   `json:"entity_id,omitempty"`
	EntityName       string   `json:"entity_name,omitempty"`
	DocumentID       string   `json:"document_id,omitempty"`
	ChunkID          string   `json:"chunk_id,omitempty"`
	ExtractionMethod string   `json:"extraction_method,omitempty"`
	Confidence       *float64 `json:"confidence,omitempty"`
}

type LineageNode struct {
	ID       string `json:"id"`
	Name     string `json:"name,omitempty"`
	NodeType string `json:"node_type,omitempty"`
}

type LineageEdge struct {
	Source       string `json:"source"`
	Target       string `json:"target"`
	Relationship string `json:"relationship,omitempty"`
}

type LineageGraph struct {
	Nodes  []LineageNode `json:"nodes"`
	Edges  []LineageEdge `json:"edges"`
	RootID string        `json:"root_id,omitempty"`
}

// DocumentLineageResponse is the response from GET /api/v1/lineage/documents/{id}.
type DocumentLineageResponse struct {
	DocumentID    string                 `json:"document_id"`
	Entities      []EntitySummary        `json:"entities"`
	Relationships []RelationshipSummary  `json:"relationships"`
	Stats         map[string]interface{} `json:"extraction_stats,omitempty"`
}

// EntitySummary is a lightweight entity in document lineage.
type EntitySummary struct {
	Name       string   `json:"entity_name"`
	Type       string   `json:"entity_type,omitempty"`
	Mentions   int      `json:"mentions,omitempty"`
	Confidence *float64 `json:"confidence,omitempty"`
}

// RelationshipSummary is a lightweight relationship in document lineage.
type RelationshipSummary struct {
	Source   string   `json:"source_entity"`
	Target   string   `json:"target_entity"`
	Keywords []string `json:"keywords,omitempty"`
	Weight   *float64 `json:"weight,omitempty"`
}

// DocumentFullLineageResponse is the response from GET /api/v1/documents/{id}/lineage.
type DocumentFullLineageResponse struct {
	DocumentID  string                 `json:"document_id"`
	Chunks      []ChunkDetail          `json:"chunks,omitempty"`
	TotalChunks int                    `json:"total_chunks,omitempty"`
	Metadata    map[string]interface{} `json:"metadata,omitempty"`
}

// ChunkLineageResponse is the response from GET /api/v1/chunks/{id}/lineage.
type ChunkLineageResponse struct {
	ChunkID       string                 `json:"chunk_id"`
	DocumentID    string                 `json:"document_id,omitempty"`
	Entities      []EntitySummary        `json:"entities,omitempty"`
	Relationships []RelationshipSummary  `json:"relationships,omitempty"`
	Metadata      map[string]interface{} `json:"metadata,omitempty"`
}

// ModelCapabilities describes what a model supports.
type ModelCapabilities struct {
	ContextLength           int  `json:"context_length"`
	MaxOutputTokens         int  `json:"max_output_tokens"`
	SupportsVision          bool `json:"supports_vision"`
	SupportsFunctionCalling bool `json:"supports_function_calling"`
	SupportsJSONMode        bool `json:"supports_json_mode"`
	SupportsStreaming       bool `json:"supports_streaming"`
	SupportsSystemMessage   bool `json:"supports_system_message"`
	EmbeddingDimension      int  `json:"embedding_dimension"`
}

// ModelCost describes per-unit pricing.
type ModelCost struct {
	InputPer1K     float64 `json:"input_per_1k"`
	OutputPer1K    float64 `json:"output_per_1k"`
	EmbeddingPer1K float64 `json:"embedding_per_1k"`
}

// ModelInfo describes a single model within a provider.
type ModelInfo struct {
	Name         string             `json:"name"`
	DisplayName  string             `json:"display_name,omitempty"`
	ModelType    string             `json:"model_type,omitempty"`
	Description  string             `json:"description,omitempty"`
	Deprecated   bool               `json:"deprecated"`
	Capabilities *ModelCapabilities `json:"capabilities,omitempty"`
	Cost         *ModelCost         `json:"cost,omitempty"`
	Provider     string             `json:"provider,omitempty"`
	IsAvailable  bool               `json:"is_available"`
}

// ProviderInfo describes an LLM provider with its models.
type ProviderInfo struct {
	Name         string      `json:"name"`
	DisplayName  string      `json:"display_name,omitempty"`
	ProviderType string      `json:"provider_type,omitempty"`
	Enabled      bool        `json:"enabled"`
	Priority     int         `json:"priority"`
	Description  string      `json:"description,omitempty"`
	Models       []ModelInfo `json:"models"`
}

// ProviderCatalog is the response from GET /api/v1/models.
type ProviderCatalog struct {
	Providers []ProviderInfo `json:"providers"`
}

type ProviderStatus struct {
	CurrentProvider string `json:"current_provider,omitempty"`
	CurrentModel    string `json:"current_model,omitempty"`
	Status          string `json:"status,omitempty"`
}

// ProviderHealthInfo describes an LLM provider health entry.
// WHY: GET /api/v1/models/health returns a bare array of these.
type ProviderHealthInfo struct {
	Name         string      `json:"name"`
	DisplayName  string      `json:"display_name,omitempty"`
	ProviderType string      `json:"provider_type,omitempty"`
	Enabled      bool        `json:"enabled"`
	Priority     int         `json:"priority"`
	Description  string      `json:"description,omitempty"`
	Models       []ModelInfo `json:"models"`
}

type CreateWorkspaceParams struct {
	Name        string `json:"name"`
	Slug        string `json:"slug,omitempty"`
	Description string `json:"description,omitempty"`
	// LLM configuration for this workspace.
	LLMModel    string `json:"llm_model,omitempty"`
	LLMProvider string `json:"llm_provider,omitempty"`
	// Embedding configuration for this workspace.
	EmbeddingModel     string `json:"embedding_model,omitempty"`
	EmbeddingProvider  string `json:"embedding_provider,omitempty"`
	EmbeddingDimension int    `json:"embedding_dimension,omitempty"`
	// Vision LLM for PDF image extraction (SPEC-041). Inherits from tenant if not set.
	VisionLLMModel    string `json:"vision_llm_model,omitempty"`
	VisionLLMProvider string `json:"vision_llm_provider,omitempty"`
}

// WorkspaceListResponse is returned by GET /api/v1/tenants/{tenant_id}/workspaces.
type WorkspaceListResponse struct {
	Items      []WorkspaceInfo `json:"items"`
	Total      int             `json:"total,omitempty"`
	Offset     int             `json:"offset,omitempty"`
	Limit      int             `json:"limit,omitempty"`
	Page       int             `json:"page,omitempty"`
	PageSize   int             `json:"page_size,omitempty"`
	TotalPages int             `json:"total_pages,omitempty"`
	HasMore    bool            `json:"has_more,omitempty"`
}

type WorkspaceInfo struct {
	ID          UUID   `json:"id"`
	Name        string `json:"name"`
	Slug        string `json:"slug,omitempty"`
	Description string `json:"description,omitempty"`
	TenantID    string `json:"tenant_id,omitempty"`
	// LLM model used for knowledge graph generation.
	LLMModel    string `json:"llm_model,omitempty"`
	LLMProvider string `json:"llm_provider,omitempty"`
	LLMFullID   string `json:"llm_full_id,omitempty"`
	// Embedding model used for vector storage.
	EmbeddingModel     string `json:"embedding_model,omitempty"`
	EmbeddingProvider  string `json:"embedding_provider,omitempty"`
	EmbeddingDimension int    `json:"embedding_dimension,omitempty"`
	EmbeddingFullID    string `json:"embedding_full_id,omitempty"`
	// Vision LLM – only present when configured or inherited from tenant.
	VisionLLMModel    string `json:"vision_llm_model,omitempty"`
	VisionLLMProvider string `json:"vision_llm_provider,omitempty"`
	CreatedAt         string `json:"created_at,omitempty"`
	UpdatedAt         string `json:"updated_at,omitempty"`
}

type WorkspaceStats struct {
	WorkspaceID       UUID  `json:"workspace_id"`
	DocumentCount     int   `json:"document_count"`
	EntityCount       int   `json:"entity_count"`
	RelationshipCount int   `json:"relationship_count"`
	ChunkCount        int   `json:"chunk_count"`
	QueryCount        int   `json:"query_count"`
	StorageSizeBytes  int64 `json:"storage_size_bytes"`
}

type RebuildResponse struct {
	Status  string `json:"status"`
	Message string `json:"message,omitempty"`
	TrackID string `json:"track_id,omitempty"`
}

type PdfProgressResponse struct {
	TrackID  string   `json:"track_id"`
	Status   string   `json:"status"`
	Progress *float64 `json:"progress,omitempty"`
}

type PdfContentResponse struct {
	ID       UUID   `json:"id"`
	Markdown string `json:"markdown,omitempty"`
}

// ============================================================================
// Paginated List Responses
// WHY: The real API wraps list results in paginated objects with different
// field names depending on the resource type.
// ============================================================================

// PaginatedList is a generic paginated response using "items" as the key.
// Used by: entities, relationships, tenants.
type PaginatedList struct {
	Items      interface{} `json:"items"`
	Total      int         `json:"total"`
	Page       int         `json:"page"`
	PageSize   int         `json:"page_size"`
	TotalPages int         `json:"total_pages"`
}

// EntityListResponse is the paginated response from GET /api/v1/graph/entities.
type EntityListResponse struct {
	Items      []Entity `json:"items"`
	Total      int      `json:"total"`
	Page       int      `json:"page"`
	PageSize   int      `json:"page_size"`
	TotalPages int      `json:"total_pages"`
}

// RelationshipListResponse is the paginated response from GET /api/v1/graph/relationships.
type RelationshipListResponse struct {
	Items      []Relationship `json:"items"`
	Total      int            `json:"total"`
	Page       int            `json:"page"`
	PageSize   int            `json:"page_size"`
	TotalPages int            `json:"total_pages"`
}

// TenantListResponse wraps paginated tenant results.
type TenantListResponse struct {
	Items      []TenantInfo `json:"items"`
	Total      int          `json:"total"`
	Page       int          `json:"page"`
	PageSize   int          `json:"page_size"`
	TotalPages int          `json:"total_pages"`
}

// UserListResponse wraps paginated user results.
type UserListResponse struct {
	Users      []UserInfo `json:"users"`
	Total      int        `json:"total"`
	Page       int        `json:"page"`
	PageSize   int        `json:"page_size"`
	TotalPages int        `json:"total_pages"`
}

// APIKeyListResponse wraps paginated API key results.
type APIKeyListResponse struct {
	Keys       []APIKeyInfo `json:"keys"`
	Total      int          `json:"total"`
	Page       int          `json:"page"`
	PageSize   int          `json:"page_size"`
	TotalPages int          `json:"total_pages"`
}

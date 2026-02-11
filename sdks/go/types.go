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
	ID      string `json:"id"`
	Status  string `json:"status,omitempty"`
	TrackID string `json:"track_id,omitempty"`
	Message string `json:"message,omitempty"`
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
	Name        string                 `json:"name"`
	EntityType  string                 `json:"entity_type,omitempty"`
	Description string                 `json:"description,omitempty"`
	Properties  map[string]interface{} `json:"properties,omitempty"`
	Degree      *int                   `json:"degree,omitempty"`
	CreatedAt   string                 `json:"created_at,omitempty"`
}

type CreateEntityParams struct {
	Name        string                 `json:"name"`
	EntityType  string                 `json:"entity_type"`
	Description string                 `json:"description,omitempty"`
	Properties  map[string]interface{} `json:"properties,omitempty"`
	SourceID    string                 `json:"source_id,omitempty"`
}

type MergeEntitiesParams struct {
	Source string `json:"source"`
	Target string `json:"target"`
}

type MergeResponse struct {
	MergedEntity *Entity `json:"merged_entity,omitempty"`
	MergedCount  int     `json:"merged_count"`
	Message      string  `json:"message,omitempty"`
}

type NeighborhoodResponse struct {
	Center *Entity     `json:"center,omitempty"`
	Nodes  []GraphNode `json:"nodes"`
	Edges  []GraphEdge `json:"edges"`
	Depth  int         `json:"depth"`
}

type EntityExistsResponse struct {
	Exists     bool   `json:"exists"`
	EntityName string `json:"entity_name,omitempty"`
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

type ChatMessage struct {
	Role    string `json:"role"`
	Content string `json:"content"`
}

type ChatCompletionRequest struct {
	Messages    []ChatMessage `json:"messages"`
	Model       string        `json:"model,omitempty"`
	Temperature *float64      `json:"temperature,omitempty"`
	MaxTokens   *int          `json:"max_tokens,omitempty"`
	Stream      *bool         `json:"stream,omitempty"`
}

type ChatCompletionResponse struct {
	ID      string       `json:"id,omitempty"`
	Choices []ChatChoice `json:"choices"`
	Model   string       `json:"model,omitempty"`
	Usage   *ChatUsage   `json:"usage,omitempty"`
}

type ChatChoice struct {
	Index        int          `json:"index"`
	Message      *ChatMessage `json:"message,omitempty"`
	FinishReason string       `json:"finish_reason,omitempty"`
}

type ChatUsage struct {
	PromptTokens     int `json:"prompt_tokens"`
	CompletionTokens int `json:"completion_tokens"`
	TotalTokens      int `json:"total_tokens"`
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
	Name string `json:"name"`
	Slug string `json:"slug,omitempty"`
}

type TenantInfo struct {
	ID   UUID   `json:"id"`
	Name string `json:"name"`
	Slug string `json:"slug,omitempty"`
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
	DeletedCount int `json:"deleted_count"`
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

type TaskInfo struct {
	TrackID    string   `json:"track_id"`
	Status     string   `json:"status"`
	Progress   *float64 `json:"progress,omitempty"`
	Message    string   `json:"message,omitempty"`
	DocumentID string   `json:"document_id,omitempty"`
	TaskType   string   `json:"task_type,omitempty"`
	CreatedAt  string   `json:"created_at,omitempty"`
	Error      string   `json:"error,omitempty"`
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

type ModelInfo struct {
	Name        string `json:"name"`
	Provider    string `json:"provider,omitempty"`
	ModelType   string `json:"model_type,omitempty"`
	IsAvailable bool   `json:"is_available"`
}

type ProviderStatus struct {
	CurrentProvider string `json:"current_provider,omitempty"`
	CurrentModel    string `json:"current_model,omitempty"`
	Status          string `json:"status,omitempty"`
}

type ProvidersHealth struct {
	Providers []ProviderHealthInfo `json:"providers"`
}

type ProviderHealthInfo struct {
	Name      string   `json:"name"`
	Status    string   `json:"status"`
	LatencyMs *float64 `json:"latency_ms,omitempty"`
	Error     string   `json:"error,omitempty"`
}

type CreateWorkspaceParams struct {
	Name        string `json:"name"`
	Slug        string `json:"slug,omitempty"`
	Description string `json:"description,omitempty"`
}

type WorkspaceInfo struct {
	ID          UUID   `json:"id"`
	Name        string `json:"name"`
	Slug        string `json:"slug,omitempty"`
	Description string `json:"description,omitempty"`
	TenantID    string `json:"tenant_id,omitempty"`
	CreatedAt   string `json:"created_at,omitempty"`
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

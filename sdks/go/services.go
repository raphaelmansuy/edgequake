package edgequake

import (
	"context"
	"fmt"
	"net/url"
	"strconv"
)

// HealthService handles /health endpoints.
type HealthService struct{ c *Client }

func (s *HealthService) Check(ctx context.Context) (*HealthResponse, error) {
	var out HealthResponse
	if err := s.c.get(ctx, "/health", nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// DocumentService handles /documents endpoints.
type DocumentService struct{ c *Client }

func (s *DocumentService) List(ctx context.Context, page, perPage int) (*ListDocumentsResponse, error) {
	params := url.Values{}
	if page > 0 {
		params.Set("page", strconv.Itoa(page))
	}
	if perPage > 0 {
		params.Set("per_page", strconv.Itoa(perPage))
	}
	var out ListDocumentsResponse
	if err := s.c.get(ctx, "/documents", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *DocumentService) Get(ctx context.Context, id string) (*Document, error) {
	var out Document
	if err := s.c.get(ctx, fmt.Sprintf("/documents/%s", id), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *DocumentService) UploadText(ctx context.Context, body map[string]interface{}) (*UploadResponse, error) {
	var out UploadResponse
	if err := s.c.post(ctx, "/documents/upload/text", body, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *DocumentService) Delete(ctx context.Context, id string) error {
	return s.c.delNoContent(ctx, fmt.Sprintf("/documents/%s", id))
}

func (s *DocumentService) Track(ctx context.Context, trackID string) (*TrackStatus, error) {
	var out TrackStatus
	if err := s.c.get(ctx, fmt.Sprintf("/documents/track/%s", trackID), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *DocumentService) Scan(ctx context.Context, params *ScanRequest) (*ScanResponse, error) {
	var out ScanResponse
	if err := s.c.post(ctx, "/documents/scan", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *DocumentService) DeletionImpact(ctx context.Context, id string) (*DeletionImpact, error) {
	var out DeletionImpact
	if err := s.c.get(ctx, fmt.Sprintf("/documents/%s/deletion-impact", id), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// GraphService handles /graph endpoints.
type GraphService struct{ c *Client }

func (s *GraphService) Get(ctx context.Context, limit int) (*GraphResponse, error) {
	params := url.Values{}
	if limit > 0 {
		params.Set("limit", strconv.Itoa(limit))
	}
	var out GraphResponse
	if err := s.c.get(ctx, "/graph", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *GraphService) Search(ctx context.Context, query string, limit int) (*SearchNodesResponse, error) {
	params := url.Values{}
	params.Set("query", query)
	if limit > 0 {
		params.Set("limit", strconv.Itoa(limit))
	}
	var out SearchNodesResponse
	if err := s.c.get(ctx, "/graph/search", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// EntityService handles /entities endpoints.
type EntityService struct{ c *Client }

func (s *EntityService) List(ctx context.Context, page, perPage int, entityType string) ([]Entity, error) {
	params := url.Values{}
	if page > 0 {
		params.Set("page", strconv.Itoa(page))
	}
	if perPage > 0 {
		params.Set("per_page", strconv.Itoa(perPage))
	}
	if entityType != "" {
		params.Set("entity_type", entityType)
	}
	var out []Entity
	if err := s.c.get(ctx, "/entities", params, &out); err != nil {
		return nil, err
	}
	return out, nil
}

func (s *EntityService) Get(ctx context.Context, name string) (*Entity, error) {
	var out Entity
	if err := s.c.get(ctx, fmt.Sprintf("/entities/%s", url.PathEscape(name)), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *EntityService) Create(ctx context.Context, params *CreateEntityParams) (*Entity, error) {
	var out Entity
	if err := s.c.post(ctx, "/entities", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *EntityService) Merge(ctx context.Context, params *MergeEntitiesParams) (*MergeResponse, error) {
	var out MergeResponse
	if err := s.c.post(ctx, "/entities/merge", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *EntityService) Delete(ctx context.Context, name string) error {
	return s.c.delNoContent(ctx, fmt.Sprintf("/entities/%s", url.PathEscape(name)))
}

func (s *EntityService) Exists(ctx context.Context, name string) (*EntityExistsResponse, error) {
	var out EntityExistsResponse
	if err := s.c.get(ctx, fmt.Sprintf("/entities/%s/exists", url.PathEscape(name)), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *EntityService) Neighborhood(ctx context.Context, name string, depth int) (*NeighborhoodResponse, error) {
	params := url.Values{}
	if depth > 0 {
		params.Set("depth", strconv.Itoa(depth))
	}
	var out NeighborhoodResponse
	if err := s.c.get(ctx, fmt.Sprintf("/entities/%s/neighborhood", url.PathEscape(name)), params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// RelationshipService handles /relationships endpoints.
type RelationshipService struct{ c *Client }

func (s *RelationshipService) List(ctx context.Context, page, perPage int) ([]Relationship, error) {
	params := url.Values{}
	if page > 0 {
		params.Set("page", strconv.Itoa(page))
	}
	if perPage > 0 {
		params.Set("per_page", strconv.Itoa(perPage))
	}
	var out []Relationship
	if err := s.c.get(ctx, "/relationships", params, &out); err != nil {
		return nil, err
	}
	return out, nil
}

func (s *RelationshipService) Create(ctx context.Context, params *CreateRelationshipParams) (*Relationship, error) {
	var out Relationship
	if err := s.c.post(ctx, "/relationships", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// QueryService handles /query endpoints.
type QueryService struct{ c *Client }

func (s *QueryService) Execute(ctx context.Context, params *QueryRequest) (*QueryResponse, error) {
	var out QueryResponse
	if err := s.c.post(ctx, "/query", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// ChatService handles /chat endpoints.
type ChatService struct{ c *Client }

func (s *ChatService) Completions(ctx context.Context, params *ChatCompletionRequest) (*ChatCompletionResponse, error) {
	var out ChatCompletionResponse
	if err := s.c.post(ctx, "/chat/completions", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// AuthService handles /auth endpoints.
type AuthService struct{ c *Client }

func (s *AuthService) Login(ctx context.Context, params *LoginParams) (*TokenResponse, error) {
	var out TokenResponse
	if err := s.c.post(ctx, "/auth/login", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *AuthService) Me(ctx context.Context) (*UserInfo, error) {
	var out UserInfo
	if err := s.c.get(ctx, "/auth/me", nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *AuthService) Refresh(ctx context.Context, params *RefreshParams) (*TokenResponse, error) {
	var out TokenResponse
	if err := s.c.post(ctx, "/auth/refresh", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// UserService handles /users endpoints.
type UserService struct{ c *Client }

func (s *UserService) Create(ctx context.Context, params *CreateUserParams) (*UserInfo, error) {
	var out UserInfo
	if err := s.c.post(ctx, "/users", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *UserService) Get(ctx context.Context, id string) (*UserInfo, error) {
	var out UserInfo
	if err := s.c.get(ctx, fmt.Sprintf("/users/%s", id), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *UserService) List(ctx context.Context) ([]UserInfo, error) {
	var out []UserInfo
	if err := s.c.get(ctx, "/users", nil, &out); err != nil {
		return nil, err
	}
	return out, nil
}

// APIKeyService handles /api-keys endpoints.
type APIKeyService struct{ c *Client }

func (s *APIKeyService) Create(ctx context.Context, name string) (*APIKeyResponse, error) {
	body := map[string]string{"name": name}
	var out APIKeyResponse
	if err := s.c.post(ctx, "/api-keys", body, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *APIKeyService) List(ctx context.Context) ([]APIKeyInfo, error) {
	var out []APIKeyInfo
	if err := s.c.get(ctx, "/api-keys", nil, &out); err != nil {
		return nil, err
	}
	return out, nil
}

func (s *APIKeyService) Revoke(ctx context.Context, id string) error {
	return s.c.delNoContent(ctx, fmt.Sprintf("/api-keys/%s", id))
}

// TenantService handles /tenants endpoints.
type TenantService struct{ c *Client }

func (s *TenantService) List(ctx context.Context) ([]TenantInfo, error) {
	var out []TenantInfo
	if err := s.c.get(ctx, "/tenants", nil, &out); err != nil {
		return nil, err
	}
	return out, nil
}

func (s *TenantService) Create(ctx context.Context, params *CreateTenantParams) (*TenantInfo, error) {
	var out TenantInfo
	if err := s.c.post(ctx, "/tenants", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// ConversationService handles /conversations endpoints.
type ConversationService struct{ c *Client }

func (s *ConversationService) Create(ctx context.Context, params *CreateConversationParams) (*ConversationInfo, error) {
	var out ConversationInfo
	if err := s.c.post(ctx, "/conversations", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *ConversationService) List(ctx context.Context) ([]ConversationInfo, error) {
	var out []ConversationInfo
	if err := s.c.get(ctx, "/conversations", nil, &out); err != nil {
		return nil, err
	}
	return out, nil
}

func (s *ConversationService) Get(ctx context.Context, id string) (*ConversationDetail, error) {
	var out ConversationDetail
	if err := s.c.get(ctx, fmt.Sprintf("/conversations/%s", id), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *ConversationService) Delete(ctx context.Context, id string) error {
	return s.c.delNoContent(ctx, fmt.Sprintf("/conversations/%s", id))
}

func (s *ConversationService) CreateMessage(ctx context.Context, conversationID string, params *CreateMessageParams) (*Message, error) {
	var out Message
	if err := s.c.post(ctx, fmt.Sprintf("/conversations/%s/messages", conversationID), params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *ConversationService) Share(ctx context.Context, id string) (*ShareLink, error) {
	var out ShareLink
	if err := s.c.post(ctx, fmt.Sprintf("/conversations/%s/share", id), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *ConversationService) BulkDelete(ctx context.Context, ids []string) (*BulkDeleteResponse, error) {
	body := map[string]interface{}{"ids": ids}
	var out BulkDeleteResponse
	if err := s.c.post(ctx, "/conversations/bulk-delete", body, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *ConversationService) Pin(ctx context.Context, id string) error {
	return s.c.postNoContent(ctx, fmt.Sprintf("/conversations/%s/pin", id), nil)
}

func (s *ConversationService) Unpin(ctx context.Context, id string) error {
	return s.c.postNoContent(ctx, fmt.Sprintf("/conversations/%s/unpin", id), nil)
}

// FolderService handles /folders endpoints.
type FolderService struct{ c *Client }

func (s *FolderService) Create(ctx context.Context, params *CreateFolderParams) (*FolderInfo, error) {
	var out FolderInfo
	if err := s.c.post(ctx, "/folders", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *FolderService) List(ctx context.Context) ([]FolderInfo, error) {
	var out []FolderInfo
	if err := s.c.get(ctx, "/folders", nil, &out); err != nil {
		return nil, err
	}
	return out, nil
}

func (s *FolderService) Get(ctx context.Context, id string) (*FolderInfo, error) {
	var out FolderInfo
	if err := s.c.get(ctx, fmt.Sprintf("/folders/%s", id), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *FolderService) Delete(ctx context.Context, id string) error {
	return s.c.delNoContent(ctx, fmt.Sprintf("/folders/%s", id))
}

// TaskService handles /tasks endpoints.
type TaskService struct{ c *Client }

func (s *TaskService) List(ctx context.Context, status string, page, perPage int) (*TaskListResponse, error) {
	params := url.Values{}
	if status != "" {
		params.Set("status", status)
	}
	if page > 0 {
		params.Set("page", strconv.Itoa(page))
	}
	if perPage > 0 {
		params.Set("per_page", strconv.Itoa(perPage))
	}
	var out TaskListResponse
	if err := s.c.get(ctx, "/tasks", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *TaskService) Get(ctx context.Context, trackID string) (*TaskInfo, error) {
	var out TaskInfo
	if err := s.c.get(ctx, fmt.Sprintf("/tasks/%s", trackID), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *TaskService) Cancel(ctx context.Context, trackID string) error {
	return s.c.postNoContent(ctx, fmt.Sprintf("/tasks/%s/cancel", trackID), nil)
}

// PipelineService handles /pipeline endpoints.
type PipelineService struct{ c *Client }

func (s *PipelineService) Status(ctx context.Context) (*PipelineStatus, error) {
	var out PipelineStatus
	if err := s.c.get(ctx, "/pipeline/status", nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *PipelineService) Metrics(ctx context.Context) (*QueueMetrics, error) {
	var out QueueMetrics
	if err := s.c.get(ctx, "/pipeline/metrics", nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// CostService handles /costs endpoints.
type CostService struct{ c *Client }

func (s *CostService) Summary(ctx context.Context) (*CostSummary, error) {
	var out CostSummary
	if err := s.c.get(ctx, "/costs/summary", nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *CostService) Breakdown(ctx context.Context, startDate, endDate string) ([]CostEntry, error) {
	params := url.Values{}
	if startDate != "" {
		params.Set("start_date", startDate)
	}
	if endDate != "" {
		params.Set("end_date", endDate)
	}
	var out []CostEntry
	if err := s.c.get(ctx, "/costs/breakdown", params, &out); err != nil {
		return nil, err
	}
	return out, nil
}

func (s *CostService) Budget(ctx context.Context) (*BudgetInfo, error) {
	var out BudgetInfo
	if err := s.c.get(ctx, "/costs/budget", nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// ChunkService handles /chunks endpoints.
type ChunkService struct{ c *Client }

func (s *ChunkService) List(ctx context.Context, documentID string, page, perPage int) ([]ChunkDetail, error) {
	params := url.Values{}
	if documentID != "" {
		params.Set("document_id", documentID)
	}
	if page > 0 {
		params.Set("page", strconv.Itoa(page))
	}
	if perPage > 0 {
		params.Set("per_page", strconv.Itoa(perPage))
	}
	var out []ChunkDetail
	if err := s.c.get(ctx, "/chunks", params, &out); err != nil {
		return nil, err
	}
	return out, nil
}

func (s *ChunkService) Get(ctx context.Context, id string) (*ChunkDetail, error) {
	var out ChunkDetail
	if err := s.c.get(ctx, fmt.Sprintf("/chunks/%s", id), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// ProvenanceService handles /provenance endpoints.
type ProvenanceService struct{ c *Client }

func (s *ProvenanceService) ForEntity(ctx context.Context, entityName string) ([]ProvenanceRecord, error) {
	var out []ProvenanceRecord
	if err := s.c.get(ctx, "/provenance/"+url.PathEscape(entityName), nil, &out); err != nil {
		return nil, err
	}
	return out, nil
}

// LineageService handles /lineage endpoints.
type LineageService struct{ c *Client }

func (s *LineageService) ForEntity(ctx context.Context, entityName string, depth int) (*LineageGraph, error) {
	params := url.Values{}
	if depth > 0 {
		params.Set("depth", strconv.Itoa(depth))
	}
	var out LineageGraph
	if err := s.c.get(ctx, fmt.Sprintf("/lineage/%s", url.PathEscape(entityName)), params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// ModelService handles /models endpoints.
type ModelService struct{ c *Client }

func (s *ModelService) List(ctx context.Context) ([]ModelInfo, error) {
	var out []ModelInfo
	if err := s.c.get(ctx, "/models", nil, &out); err != nil {
		return nil, err
	}
	return out, nil
}

func (s *ModelService) SetProvider(ctx context.Context, provider string) (*ProviderStatus, error) {
	body := map[string]string{"provider": provider}
	var out ProviderStatus
	if err := s.c.post(ctx, "/models/provider", body, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *ModelService) ProviderHealth(ctx context.Context) (*ProvidersHealth, error) {
	var out ProvidersHealth
	if err := s.c.get(ctx, "/models/health", nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// WorkspaceService handles /workspaces endpoints.
type WorkspaceService struct{ c *Client }

func (s *WorkspaceService) List(ctx context.Context) ([]WorkspaceInfo, error) {
	var out []WorkspaceInfo
	if err := s.c.get(ctx, "/workspaces", nil, &out); err != nil {
		return nil, err
	}
	return out, nil
}

func (s *WorkspaceService) Create(ctx context.Context, params *CreateWorkspaceParams) (*WorkspaceInfo, error) {
	var out WorkspaceInfo
	if err := s.c.post(ctx, "/workspaces", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *WorkspaceService) Get(ctx context.Context, id string) (*WorkspaceInfo, error) {
	var out WorkspaceInfo
	if err := s.c.get(ctx, fmt.Sprintf("/workspaces/%s", id), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *WorkspaceService) Stats(ctx context.Context, id string) (*WorkspaceStats, error) {
	var out WorkspaceStats
	if err := s.c.get(ctx, fmt.Sprintf("/workspaces/%s/stats", id), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *WorkspaceService) Rebuild(ctx context.Context, id string) (*RebuildResponse, error) {
	var out RebuildResponse
	if err := s.c.post(ctx, fmt.Sprintf("/workspaces/%s/rebuild", id), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// PDFService handles /pdf endpoints.
type PDFService struct{ c *Client }

func (s *PDFService) Progress(ctx context.Context, documentID string) (*PdfProgressResponse, error) {
	var out PdfProgressResponse
	if err := s.c.get(ctx, fmt.Sprintf("/pdf/%s/progress", documentID), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *PDFService) Content(ctx context.Context, documentID string) (*PdfContentResponse, error) {
	var out PdfContentResponse
	if err := s.c.get(ctx, fmt.Sprintf("/pdf/%s/content", documentID), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

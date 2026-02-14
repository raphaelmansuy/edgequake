package edgequake

import (
	"context"
	"fmt"
	"net/url"
	"strconv"
)

// WHY: All business endpoints live under /api/v1/ in the real EdgeQuake API.
// Only /health, /ready, /live are at the root level.
// Entities and relationships are under /api/v1/graph/.
// These paths were verified against edgequake/crates/edgequake-api/src/routes.rs.

// HealthService handles root-level health endpoints.
type HealthService struct{ c *Client }

func (s *HealthService) Check(ctx context.Context) (*HealthResponse, error) {
	var out HealthResponse
	if err := s.c.get(ctx, "/health", nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// DocumentService handles /api/v1/documents endpoints.
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
	if err := s.c.get(ctx, "/api/v1/documents", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *DocumentService) Get(ctx context.Context, id string) (*Document, error) {
	var out Document
	if err := s.c.get(ctx, fmt.Sprintf("/api/v1/documents/%s", id), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// WHY: POST /api/v1/documents is the text upload handler (upload_document).
// File upload uses /api/v1/documents/upload (multipart).
func (s *DocumentService) UploadText(ctx context.Context, body map[string]interface{}) (*UploadResponse, error) {
	var out UploadResponse
	if err := s.c.post(ctx, "/api/v1/documents", body, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *DocumentService) Delete(ctx context.Context, id string) error {
	return s.c.delNoContent(ctx, fmt.Sprintf("/api/v1/documents/%s", id))
}

func (s *DocumentService) DeleteAll(ctx context.Context) error {
	return s.c.delNoContent(ctx, "/api/v1/documents")
}

func (s *DocumentService) Track(ctx context.Context, trackID string) (*TrackStatus, error) {
	var out TrackStatus
	if err := s.c.get(ctx, fmt.Sprintf("/api/v1/documents/track/%s", trackID), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *DocumentService) Scan(ctx context.Context, params *ScanRequest) (*ScanResponse, error) {
	var out ScanResponse
	if err := s.c.post(ctx, "/api/v1/documents/scan", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *DocumentService) DeletionImpact(ctx context.Context, id string) (*DeletionImpact, error) {
	var out DeletionImpact
	if err := s.c.get(ctx, fmt.Sprintf("/api/v1/documents/%s/deletion-impact", id), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// GraphService handles /api/v1/graph endpoints.
type GraphService struct{ c *Client }

func (s *GraphService) Get(ctx context.Context, limit int) (*GraphResponse, error) {
	params := url.Values{}
	if limit > 0 {
		params.Set("limit", strconv.Itoa(limit))
	}
	var out GraphResponse
	if err := s.c.get(ctx, "/api/v1/graph", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// WHY: Search uses /api/v1/graph/nodes/search per routes.rs (search_nodes handler).
func (s *GraphService) Search(ctx context.Context, query string, limit int) (*SearchNodesResponse, error) {
	params := url.Values{}
	params.Set("q", query)
	if limit > 0 {
		params.Set("limit", strconv.Itoa(limit))
	}
	var out SearchNodesResponse
	if err := s.c.get(ctx, "/api/v1/graph/nodes/search", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// EntityService handles /api/v1/graph/entities endpoints.
// WHY: Entities are nested under /graph/ in the real API, not at /entities/.
type EntityService struct{ c *Client }

func (s *EntityService) List(ctx context.Context, page, perPage int, entityType string) (*EntityListResponse, error) {
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
	var out EntityListResponse
	if err := s.c.get(ctx, "/api/v1/graph/entities", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *EntityService) Get(ctx context.Context, name string) (*EntityDetailResponse, error) {
	var out EntityDetailResponse
	if err := s.c.get(ctx, fmt.Sprintf("/api/v1/graph/entities/%s", url.PathEscape(name)), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *EntityService) Create(ctx context.Context, params *CreateEntityParams) (*CreateEntityResponse, error) {
	var out CreateEntityResponse
	if err := s.c.post(ctx, "/api/v1/graph/entities", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *EntityService) Merge(ctx context.Context, params *MergeEntitiesParams) (*MergeResponse, error) {
	var out MergeResponse
	if err := s.c.post(ctx, "/api/v1/graph/entities/merge", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *EntityService) Delete(ctx context.Context, name string) error {
	return s.c.delNoContent(ctx, fmt.Sprintf("/api/v1/graph/entities/%s?confirm=true", url.PathEscape(name)))
}

// WHY: Entity exists check uses query param, not path segment.
// GET /api/v1/graph/entities/exists?entity_name=NAME
func (s *EntityService) Exists(ctx context.Context, name string) (*EntityExistsResponse, error) {
	params := url.Values{}
	params.Set("entity_name", name)
	var out EntityExistsResponse
	if err := s.c.get(ctx, "/api/v1/graph/entities/exists", params, &out); err != nil {
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
	if err := s.c.get(ctx, fmt.Sprintf("/api/v1/graph/entities/%s/neighborhood", url.PathEscape(name)), params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// RelationshipService handles /api/v1/graph/relationships endpoints.
// WHY: Relationships are nested under /graph/ in the real API.
type RelationshipService struct{ c *Client }

func (s *RelationshipService) List(ctx context.Context, page, perPage int) (*RelationshipListResponse, error) {
	params := url.Values{}
	if page > 0 {
		params.Set("page", strconv.Itoa(page))
	}
	if perPage > 0 {
		params.Set("per_page", strconv.Itoa(perPage))
	}
	var out RelationshipListResponse
	if err := s.c.get(ctx, "/api/v1/graph/relationships", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *RelationshipService) Create(ctx context.Context, params *CreateRelationshipParams) (*Relationship, error) {
	var out Relationship
	if err := s.c.post(ctx, "/api/v1/graph/relationships", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// QueryService handles /api/v1/query endpoints.
type QueryService struct{ c *Client }

func (s *QueryService) Execute(ctx context.Context, params *QueryRequest) (*QueryResponse, error) {
	var out QueryResponse
	if err := s.c.post(ctx, "/api/v1/query", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// ChatService handles /api/v1/chat endpoints.
type ChatService struct{ c *Client }

func (s *ChatService) Completions(ctx context.Context, params *ChatCompletionRequest) (*ChatCompletionResponse, error) {
	var out ChatCompletionResponse
	if err := s.c.post(ctx, "/api/v1/chat/completions", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// AuthService handles /api/v1/auth endpoints.
type AuthService struct{ c *Client }

func (s *AuthService) Login(ctx context.Context, params *LoginParams) (*TokenResponse, error) {
	var out TokenResponse
	if err := s.c.post(ctx, "/api/v1/auth/login", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *AuthService) Me(ctx context.Context) (*UserInfo, error) {
	var out UserInfo
	if err := s.c.get(ctx, "/api/v1/auth/me", nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *AuthService) Refresh(ctx context.Context, params *RefreshParams) (*TokenResponse, error) {
	var out TokenResponse
	if err := s.c.post(ctx, "/api/v1/auth/refresh", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// UserService handles /api/v1/users endpoints.
type UserService struct{ c *Client }

func (s *UserService) Create(ctx context.Context, params *CreateUserParams) (*UserInfo, error) {
	var out UserInfo
	if err := s.c.post(ctx, "/api/v1/users", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *UserService) Get(ctx context.Context, id string) (*UserInfo, error) {
	var out UserInfo
	if err := s.c.get(ctx, fmt.Sprintf("/api/v1/users/%s", id), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *UserService) List(ctx context.Context) (*UserListResponse, error) {
	var out UserListResponse
	if err := s.c.get(ctx, "/api/v1/users", nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// APIKeyService handles /api/v1/api-keys endpoints.
type APIKeyService struct{ c *Client }

func (s *APIKeyService) Create(ctx context.Context, name string) (*APIKeyResponse, error) {
	body := map[string]string{"name": name}
	var out APIKeyResponse
	if err := s.c.post(ctx, "/api/v1/api-keys", body, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *APIKeyService) List(ctx context.Context) (*APIKeyListResponse, error) {
	var out APIKeyListResponse
	if err := s.c.get(ctx, "/api/v1/api-keys", nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *APIKeyService) Revoke(ctx context.Context, id string) error {
	return s.c.delNoContent(ctx, fmt.Sprintf("/api/v1/api-keys/%s", id))
}

// TenantService handles /api/v1/tenants endpoints.
type TenantService struct{ c *Client }

func (s *TenantService) List(ctx context.Context) (*TenantListResponse, error) {
	var out TenantListResponse
	if err := s.c.get(ctx, "/api/v1/tenants", nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *TenantService) Create(ctx context.Context, params *CreateTenantParams) (*TenantInfo, error) {
	var out TenantInfo
	if err := s.c.post(ctx, "/api/v1/tenants", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// ConversationService handles /api/v1/conversations endpoints.
type ConversationService struct{ c *Client }

func (s *ConversationService) Create(ctx context.Context, params *CreateConversationParams) (*ConversationInfo, error) {
	var out ConversationInfo
	if err := s.c.post(ctx, "/api/v1/conversations", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *ConversationService) List(ctx context.Context) ([]ConversationInfo, error) {
	// WHY: API returns {items: [...], pagination: {...}}, not a raw array
	var wrapper struct {
		Items []ConversationInfo `json:"items"`
	}
	if err := s.c.get(ctx, "/api/v1/conversations", nil, &wrapper); err != nil {
		return nil, err
	}
	return wrapper.Items, nil
}

func (s *ConversationService) Get(ctx context.Context, id string) (*ConversationDetail, error) {
	var out ConversationDetail
	if err := s.c.get(ctx, fmt.Sprintf("/api/v1/conversations/%s", id), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *ConversationService) Delete(ctx context.Context, id string) error {
	return s.c.delNoContent(ctx, fmt.Sprintf("/api/v1/conversations/%s", id))
}

func (s *ConversationService) CreateMessage(ctx context.Context, conversationID string, params *CreateMessageParams) (*Message, error) {
	var out Message
	if err := s.c.post(ctx, fmt.Sprintf("/api/v1/conversations/%s/messages", conversationID), params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *ConversationService) Share(ctx context.Context, id string) (*ShareLink, error) {
	var out ShareLink
	if err := s.c.post(ctx, fmt.Sprintf("/api/v1/conversations/%s/share", id), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// WHY: Bulk delete uses /api/v1/conversations/bulk/delete (not bulk-delete).
// Verified against routes.rs: .route("/conversations/bulk/delete", post(...))
func (s *ConversationService) BulkDelete(ctx context.Context, ids []string) (*BulkDeleteResponse, error) {
	body := map[string]interface{}{"ids": ids}
	var out BulkDeleteResponse
	if err := s.c.post(ctx, "/api/v1/conversations/bulk/delete", body, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// WHY: Pin/unpin via PATCH /api/v1/conversations/{id} with is_pinned field.
// The actual API uses update_conversation handler for pin state changes.
func (s *ConversationService) Pin(ctx context.Context, id string) error {
	body := map[string]interface{}{"is_pinned": true}
	return s.c.patchNoContent(ctx, fmt.Sprintf("/api/v1/conversations/%s", id), body)
}

func (s *ConversationService) Unpin(ctx context.Context, id string) error {
	body := map[string]interface{}{"is_pinned": false}
	return s.c.patchNoContent(ctx, fmt.Sprintf("/api/v1/conversations/%s", id), body)
}

// FolderService handles /api/v1/folders endpoints.
type FolderService struct{ c *Client }

func (s *FolderService) Create(ctx context.Context, params *CreateFolderParams) (*FolderInfo, error) {
	var out FolderInfo
	if err := s.c.post(ctx, "/api/v1/folders", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *FolderService) List(ctx context.Context) ([]FolderInfo, error) {
	var out []FolderInfo
	if err := s.c.get(ctx, "/api/v1/folders", nil, &out); err != nil {
		return nil, err
	}
	return out, nil
}

func (s *FolderService) Get(ctx context.Context, id string) (*FolderInfo, error) {
	var out FolderInfo
	if err := s.c.get(ctx, fmt.Sprintf("/api/v1/folders/%s", id), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *FolderService) Delete(ctx context.Context, id string) error {
	return s.c.delNoContent(ctx, fmt.Sprintf("/api/v1/folders/%s", id))
}

// TaskService handles /api/v1/tasks endpoints.
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
	if err := s.c.get(ctx, "/api/v1/tasks", params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *TaskService) Get(ctx context.Context, trackID string) (*TaskInfo, error) {
	var out TaskInfo
	if err := s.c.get(ctx, fmt.Sprintf("/api/v1/tasks/%s", trackID), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *TaskService) Cancel(ctx context.Context, trackID string) error {
	return s.c.postNoContent(ctx, fmt.Sprintf("/api/v1/tasks/%s/cancel", trackID), nil)
}

// PipelineService handles /api/v1/pipeline endpoints.
type PipelineService struct{ c *Client }

func (s *PipelineService) Status(ctx context.Context) (*PipelineStatus, error) {
	var out PipelineStatus
	if err := s.c.get(ctx, "/api/v1/pipeline/status", nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// WHY: The actual route is /api/v1/pipeline/queue-metrics, not /pipeline/metrics.
func (s *PipelineService) Metrics(ctx context.Context) (*QueueMetrics, error) {
	var out QueueMetrics
	if err := s.c.get(ctx, "/api/v1/pipeline/queue-metrics", nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// CostService handles /api/v1/costs endpoints.
type CostService struct{ c *Client }

func (s *CostService) Summary(ctx context.Context) (*CostSummary, error) {
	var out CostSummary
	if err := s.c.get(ctx, "/api/v1/costs/summary", nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// WHY: The actual route is /api/v1/costs/history, not /costs/breakdown.
func (s *CostService) History(ctx context.Context, startDate, endDate string) ([]CostEntry, error) {
	params := url.Values{}
	if startDate != "" {
		params.Set("start_date", startDate)
	}
	if endDate != "" {
		params.Set("end_date", endDate)
	}
	var out []CostEntry
	if err := s.c.get(ctx, "/api/v1/costs/history", params, &out); err != nil {
		return nil, err
	}
	return out, nil
}

func (s *CostService) Budget(ctx context.Context) (*BudgetInfo, error) {
	var out BudgetInfo
	if err := s.c.get(ctx, "/api/v1/costs/budget", nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// ChunkService handles /api/v1/chunks endpoints.
// WHY: Only GET /api/v1/chunks/{chunk_id} exists. There's no list-all-chunks route.
type ChunkService struct{ c *Client }

func (s *ChunkService) Get(ctx context.Context, id string) (*ChunkDetail, error) {
	var out ChunkDetail
	if err := s.c.get(ctx, fmt.Sprintf("/api/v1/chunks/%s", id), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// Lineage returns chunk lineage with parent document references.
// WHY: Route is /api/v1/chunks/{id}/lineage per routes.rs.
func (s *ChunkService) Lineage(ctx context.Context, id string) (*ChunkLineageResponse, error) {
	var out ChunkLineageResponse
	if err := s.c.get(ctx, fmt.Sprintf("/api/v1/chunks/%s/lineage", id), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// ProvenanceService handles entity provenance.
// WHY: Route is /api/v1/entities/{entity_id}/provenance per routes.rs.
type ProvenanceService struct{ c *Client }

func (s *ProvenanceService) ForEntity(ctx context.Context, entityID string) ([]ProvenanceRecord, error) {
	var out []ProvenanceRecord
	if err := s.c.get(ctx, fmt.Sprintf("/api/v1/entities/%s/provenance", url.PathEscape(entityID)), nil, &out); err != nil {
		return nil, err
	}
	return out, nil
}

// LineageService handles /api/v1/lineage endpoints.
type LineageService struct{ c *Client }

// WHY: Route is /api/v1/lineage/entities/{entity_name} per routes.rs.
func (s *LineageService) ForEntity(ctx context.Context, entityName string, depth int) (*LineageGraph, error) {
	params := url.Values{}
	if depth > 0 {
		params.Set("depth", strconv.Itoa(depth))
	}
	var out LineageGraph
	if err := s.c.get(ctx, fmt.Sprintf("/api/v1/lineage/entities/%s", url.PathEscape(entityName)), params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// ForDocument returns document graph lineage with entities and relationships.
// WHY: Route is /api/v1/lineage/documents/{id} per routes.rs.
func (s *LineageService) ForDocument(ctx context.Context, documentID string) (*DocumentLineageResponse, error) {
	var out DocumentLineageResponse
	if err := s.c.get(ctx, fmt.Sprintf("/api/v1/lineage/documents/%s", documentID), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// DocumentFullLineage returns the full document lineage including chunk details.
// WHY: Route is /api/v1/documents/{id}/lineage per routes.rs.
func (s *LineageService) DocumentFullLineage(ctx context.Context, documentID string) (*DocumentFullLineageResponse, error) {
	var out DocumentFullLineageResponse
	if err := s.c.get(ctx, fmt.Sprintf("/api/v1/documents/%s/lineage", documentID), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// ExportLineage exports document lineage as JSON or CSV. Returns raw bytes.
// WHY: Route is /api/v1/documents/{id}/lineage/export?format= per routes.rs.
func (s *LineageService) ExportLineage(ctx context.Context, documentID, format string) ([]byte, error) {
	params := url.Values{}
	if format != "" {
		params.Set("format", format)
	}
	return s.c.getRaw(ctx, fmt.Sprintf("/api/v1/documents/%s/lineage/export", documentID), params)
}

// ModelService handles /api/v1/models endpoints.
type ModelService struct{ c *Client }

func (s *ModelService) List(ctx context.Context) (*ProviderCatalog, error) {
	var out ProviderCatalog
	if err := s.c.get(ctx, "/api/v1/models", nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// WHY: Provider status is at /api/v1/settings/provider/status (GET).
func (s *ModelService) ProviderStatus(ctx context.Context) (*ProviderStatus, error) {
	var out ProviderStatus
	if err := s.c.get(ctx, "/api/v1/settings/provider/status", nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// WHY: GET /api/v1/models/health returns a bare array of provider health objects.
func (s *ModelService) ProviderHealth(ctx context.Context) ([]ProviderHealthInfo, error) {
	var out []ProviderHealthInfo
	if err := s.c.get(ctx, "/api/v1/models/health", nil, &out); err != nil {
		return nil, err
	}
	return out, nil
}

// WorkspaceService handles /api/v1/workspaces endpoints.
// WHY: List/Create are tenant-scoped: /api/v1/tenants/{tenant_id}/workspaces.
// Get/Stats are direct: /api/v1/workspaces/{workspace_id}.
type WorkspaceService struct{ c *Client }

func (s *WorkspaceService) ListForTenant(ctx context.Context, tenantID string) ([]WorkspaceInfo, error) {
	var out []WorkspaceInfo
	if err := s.c.get(ctx, fmt.Sprintf("/api/v1/tenants/%s/workspaces", tenantID), nil, &out); err != nil {
		return nil, err
	}
	return out, nil
}

func (s *WorkspaceService) CreateForTenant(ctx context.Context, tenantID string, params *CreateWorkspaceParams) (*WorkspaceInfo, error) {
	var out WorkspaceInfo
	if err := s.c.post(ctx, fmt.Sprintf("/api/v1/tenants/%s/workspaces", tenantID), params, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *WorkspaceService) Get(ctx context.Context, id string) (*WorkspaceInfo, error) {
	var out WorkspaceInfo
	if err := s.c.get(ctx, fmt.Sprintf("/api/v1/workspaces/%s", id), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *WorkspaceService) Stats(ctx context.Context, id string) (*WorkspaceStats, error) {
	var out WorkspaceStats
	if err := s.c.get(ctx, fmt.Sprintf("/api/v1/workspaces/%s/stats", id), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *WorkspaceService) RebuildEmbeddings(ctx context.Context, id string) (*RebuildResponse, error) {
	var out RebuildResponse
	if err := s.c.post(ctx, fmt.Sprintf("/api/v1/workspaces/%s/rebuild-embeddings", id), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// PDFService handles /api/v1/documents/pdf endpoints.
// WHY: PDF endpoints are nested under /documents/pdf/ in the real API.
type PDFService struct{ c *Client }

func (s *PDFService) Progress(ctx context.Context, trackID string) (*PdfProgressResponse, error) {
	var out PdfProgressResponse
	if err := s.c.get(ctx, fmt.Sprintf("/api/v1/documents/pdf/progress/%s", trackID), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *PDFService) Content(ctx context.Context, pdfID string) (*PdfContentResponse, error) {
	var out PdfContentResponse
	if err := s.c.get(ctx, fmt.Sprintf("/api/v1/documents/pdf/%s/content", pdfID), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *PDFService) Status(ctx context.Context, pdfID string) (*PdfProgressResponse, error) {
	var out PdfProgressResponse
	if err := s.c.get(ctx, fmt.Sprintf("/api/v1/documents/pdf/%s", pdfID), nil, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func (s *PDFService) List(ctx context.Context) ([]Document, error) {
	var out []Document
	if err := s.c.get(ctx, "/api/v1/documents/pdf", nil, &out); err != nil {
		return nil, err
	}
	return out, nil
}

package edgequake_test

import (
	"context"
	"encoding/json"
	"errors"
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"sync/atomic"
	"testing"
	"time"

	edgequake "github.com/edgequake/edgequake-go"
)

// ── Helpers ──────────────────────────────────────────────────────────────────

// routedServer lets tests assert the HTTP method, path, query, headers, and
// body that the SDK actually sends.
func routedServer(t *testing.T, handler func(w http.ResponseWriter, r *http.Request)) *httptest.Server {
	t.Helper()
	return httptest.NewServer(http.HandlerFunc(handler))
}

func jsonOK(w http.ResponseWriter, v interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(200)
	_ = json.NewEncoder(w).Encode(v)
}

func jsonStatus(w http.ResponseWriter, code int, v interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(code)
	if v != nil {
		_ = json.NewEncoder(w).Encode(v)
	}
}

func noContent(w http.ResponseWriter) {
	w.WriteHeader(204)
}

// ── Option Tests ─────────────────────────────────────────────────────────────

func TestWithBearerToken(t *testing.T) {
	var got string
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		got = r.Header.Get("Authorization")
		jsonOK(w, edgequake.HealthResponse{Status: "ok"})
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithBearerToken("jwt-xyz"))
	_, _ = c.Health.Check(context.Background())
	if got != "Bearer jwt-xyz" {
		t.Fatalf("expected Bearer jwt-xyz, got %q", got)
	}
}

func TestWithUserID(t *testing.T) {
	var got string
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		got = r.Header.Get("X-User-ID")
		jsonOK(w, []edgequake.ConversationInfo{})
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithUserID("u-42"))
	_, _ = c.Conversations.List(context.Background())
	if got != "u-42" {
		t.Fatalf("expected u-42, got %q", got)
	}
}

func TestWithTimeout(t *testing.T) {
	c := edgequake.NewClient(edgequake.WithTimeout(5 * time.Second))
	if c == nil {
		t.Fatal("nil client")
	}
}

// ── Document Service ─────────────────────────────────────────────────────────

func TestDocuments_DeleteAll(t *testing.T) {
	srv := mockServer(t, 204, nil)
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	if err := c.Documents.DeleteAll(context.Background()); err != nil {
		t.Fatal(err)
	}
}

func TestDocuments_DeletionImpact(t *testing.T) {
	srv := mockServer(t, 200, edgequake.DeletionImpact{
		EntityCount: 5, RelationshipCount: 10, ChunkCount: 20,
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	impact, err := c.Documents.DeletionImpact(context.Background(), "d1")
	if err != nil {
		t.Fatal(err)
	}
	if impact.EntityCount != 5 {
		t.Fatalf("got %d", impact.EntityCount)
	}
	if impact.RelationshipCount != 10 {
		t.Fatalf("got %d", impact.RelationshipCount)
	}
}

func TestDocuments_List_WithPagination(t *testing.T) {
	var gotPath string
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		gotPath = r.URL.String()
		jsonOK(w, edgequake.ListDocumentsResponse{})
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, _ = c.Documents.List(context.Background(), 2, 25)
	if !strings.Contains(gotPath, "page=2") || !strings.Contains(gotPath, "per_page=25") {
		t.Fatalf("expected page/per_page params, got %s", gotPath)
	}
}

func TestDocuments_List_NoParams(t *testing.T) {
	var gotPath string
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		gotPath = r.URL.String()
		jsonOK(w, edgequake.ListDocumentsResponse{})
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, _ = c.Documents.List(context.Background(), 0, 0)
	if strings.Contains(gotPath, "page=") || strings.Contains(gotPath, "per_page=") {
		t.Fatalf("should not include page params for zero values, got %s", gotPath)
	}
}

// ── Entity Service ───────────────────────────────────────────────────────────

func TestEntities_Get(t *testing.T) {
	srv := mockServer(t, 200, edgequake.EntityDetailResponse{
		Entity: &edgequake.Entity{EntityName: "SARAH_CHEN", EntityType: "PERSON"},
		Statistics: &edgequake.EntityStatistics{
			TotalRelationships: 5,
			OutgoingCount:      3,
			IncomingCount:      2,
		},
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	detail, err := c.Entities.Get(context.Background(), "SARAH_CHEN")
	if err != nil {
		t.Fatal(err)
	}
	if detail.Entity.EntityName != "SARAH_CHEN" {
		t.Fatalf("got %s", detail.Entity.EntityName)
	}
	if detail.Statistics.TotalRelationships != 5 {
		t.Fatalf("got %d", detail.Statistics.TotalRelationships)
	}
}

func TestEntities_Exists(t *testing.T) {
	entityID := "e-123"
	srv := mockServer(t, 200, edgequake.EntityExistsResponse{
		Exists:   true,
		EntityID: &entityID,
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.Entities.Exists(context.Background(), "TEST")
	if err != nil {
		t.Fatal(err)
	}
	if !resp.Exists {
		t.Fatal("expected exists=true")
	}
	if *resp.EntityID != "e-123" {
		t.Fatalf("got %s", *resp.EntityID)
	}
}

func TestEntities_Exists_NotFound(t *testing.T) {
	srv := mockServer(t, 200, edgequake.EntityExistsResponse{Exists: false})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.Entities.Exists(context.Background(), "MISSING")
	if err != nil {
		t.Fatal(err)
	}
	if resp.Exists {
		t.Fatal("expected exists=false")
	}
}

func TestEntities_List_WithFilters(t *testing.T) {
	var gotPath string
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		gotPath = r.URL.String()
		jsonOK(w, edgequake.EntityListResponse{})
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, _ = c.Entities.List(context.Background(), 1, 20, "PERSON")
	if !strings.Contains(gotPath, "entity_type=PERSON") {
		t.Fatalf("expected entity_type param, got %s", gotPath)
	}
	if !strings.Contains(gotPath, "page=1") {
		t.Fatalf("expected page param, got %s", gotPath)
	}
}

func TestEntities_Neighborhood_NoDepth(t *testing.T) {
	srv := mockServer(t, 200, edgequake.NeighborhoodResponse{
		Nodes: []edgequake.GraphNode{{ID: "n1"}},
		Depth: 1,
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.Entities.Neighborhood(context.Background(), "E1", 0)
	if err != nil {
		t.Fatal(err)
	}
	if len(resp.Nodes) != 1 {
		t.Fatalf("got %d", len(resp.Nodes))
	}
}

// ── Auth Service ─────────────────────────────────────────────────────────────

func TestAuth_Refresh(t *testing.T) {
	srv := mockServer(t, 200, edgequake.TokenResponse{
		AccessToken:  "new-tok",
		RefreshToken: "new-refresh",
		TokenType:    "Bearer",
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	tok, err := c.Auth.Refresh(context.Background(), &edgequake.RefreshParams{
		RefreshToken: "old-refresh",
	})
	if err != nil {
		t.Fatal(err)
	}
	if tok.AccessToken != "new-tok" {
		t.Fatalf("got %s", tok.AccessToken)
	}
}

// ── User Service ─────────────────────────────────────────────────────────────

func TestUsers_Get(t *testing.T) {
	srv := mockServer(t, 200, edgequake.UserInfo{ID: "u1", Username: "alice", Email: "a@b.com"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	u, err := c.Users.Get(context.Background(), "u1")
	if err != nil {
		t.Fatal(err)
	}
	if u.Username != "alice" {
		t.Fatalf("got %s", u.Username)
	}
}

func TestUsers_List(t *testing.T) {
	srv := mockServer(t, 200, edgequake.UserListResponse{
		Users: []edgequake.UserInfo{{ID: "u1"}, {ID: "u2"}},
		Total: 2,
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.Users.List(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if resp.Total != 2 {
		t.Fatalf("got %d", resp.Total)
	}
}

// ── APIKey Service ───────────────────────────────────────────────────────────

func TestAPIKeys_List(t *testing.T) {
	srv := mockServer(t, 200, edgequake.APIKeyListResponse{
		Keys:  []edgequake.APIKeyInfo{{ID: "k1", Name: "key1"}},
		Total: 1,
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.APIKeys.List(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if resp.Total != 1 {
		t.Fatalf("got %d", resp.Total)
	}
}

// ── Conversation Service ─────────────────────────────────────────────────────

func TestConversations_List(t *testing.T) {
	// WHY: API returns {"items":[...]} wrapper, not raw array.
	srv := mockServer(t, 200, struct {
		Items []edgequake.ConversationInfo `json:"items"`
	}{
		Items: []edgequake.ConversationInfo{
			{ID: "c1", Title: "Chat 1"},
			{ID: "c2", Title: "Chat 2"},
		},
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	convs, err := c.Conversations.List(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if len(convs) != 2 {
		t.Fatalf("got %d", len(convs))
	}
}

func TestConversations_Get(t *testing.T) {
	srv := mockServer(t, 200, edgequake.ConversationDetail{
		ID:    "c1",
		Title: "Test Chat",
		Messages: []edgequake.Message{
			{ID: "m1", Role: "user", Content: "Hello"},
		},
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	detail, err := c.Conversations.Get(context.Background(), "c1")
	if err != nil {
		t.Fatal(err)
	}
	if len(detail.Messages) != 1 {
		t.Fatalf("got %d", len(detail.Messages))
	}
}

func TestConversations_Delete(t *testing.T) {
	srv := mockServer(t, 204, nil)
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	if err := c.Conversations.Delete(context.Background(), "c1"); err != nil {
		t.Fatal(err)
	}
}

func TestConversations_Pin(t *testing.T) {
	var gotMethod, gotPath string
	var gotBody map[string]interface{}
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		gotMethod = r.Method
		gotPath = r.URL.Path
		b, _ := io.ReadAll(r.Body)
		_ = json.Unmarshal(b, &gotBody)
		w.WriteHeader(204)
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	if err := c.Conversations.Pin(context.Background(), "c1"); err != nil {
		t.Fatal(err)
	}
	if gotMethod != "PATCH" {
		t.Fatalf("expected PATCH, got %s", gotMethod)
	}
	if !strings.Contains(gotPath, "/conversations/c1") {
		t.Fatalf("expected path with /conversations/c1, got %s", gotPath)
	}
	if gotBody["is_pinned"] != true {
		t.Fatalf("expected is_pinned=true, got %v", gotBody["is_pinned"])
	}
}

func TestConversations_Unpin(t *testing.T) {
	var gotBody map[string]interface{}
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		b, _ := io.ReadAll(r.Body)
		_ = json.Unmarshal(b, &gotBody)
		w.WriteHeader(204)
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	if err := c.Conversations.Unpin(context.Background(), "c1"); err != nil {
		t.Fatal(err)
	}
	if gotBody["is_pinned"] != false {
		t.Fatalf("expected is_pinned=false, got %v", gotBody["is_pinned"])
	}
}

// ── Folder Service ───────────────────────────────────────────────────────────

func TestFolders_List(t *testing.T) {
	srv := mockServer(t, 200, []edgequake.FolderInfo{
		{ID: "f1", Name: "Folder 1"},
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	folders, err := c.Folders.List(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if len(folders) != 1 {
		t.Fatalf("got %d", len(folders))
	}
}

func TestFolders_Get(t *testing.T) {
	srv := mockServer(t, 200, edgequake.FolderInfo{ID: "f1", Name: "Research"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	f, err := c.Folders.Get(context.Background(), "f1")
	if err != nil {
		t.Fatal(err)
	}
	if f.Name != "Research" {
		t.Fatalf("got %s", f.Name)
	}
}

func TestFolders_Delete(t *testing.T) {
	srv := mockServer(t, 204, nil)
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	if err := c.Folders.Delete(context.Background(), "f1"); err != nil {
		t.Fatal(err)
	}
}

// ── Task Service ─────────────────────────────────────────────────────────────

func TestTasks_Get(t *testing.T) {
	srv := mockServer(t, 200, edgequake.TaskInfo{
		TrackID: "tk1",
		Status:  "completed",
		Result: &edgequake.TaskResult{
			EntityCount: 10,
			ChunkCount:  5,
		},
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	task, err := c.Tasks.Get(context.Background(), "tk1")
	if err != nil {
		t.Fatal(err)
	}
	if task.Status != "completed" {
		t.Fatalf("got %s", task.Status)
	}
	if task.Result.EntityCount != 10 {
		t.Fatalf("got %d", task.Result.EntityCount)
	}
}

func TestTasks_List_WithFilters(t *testing.T) {
	var gotPath string
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		gotPath = r.URL.String()
		jsonOK(w, edgequake.TaskListResponse{})
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, _ = c.Tasks.List(context.Background(), "running", 2, 50)
	if !strings.Contains(gotPath, "status=running") {
		t.Fatalf("expected status param, got %s", gotPath)
	}
	if !strings.Contains(gotPath, "page=2") {
		t.Fatalf("expected page param, got %s", gotPath)
	}
}

// ── Cost Service ─────────────────────────────────────────────────────────────

func TestCosts_History(t *testing.T) {
	srv := mockServer(t, 200, []edgequake.CostEntry{
		{Date: "2024-01-01", CostUSD: 0.50, Tokens: 1000, Requests: 10},
		{Date: "2024-01-02", CostUSD: 0.75, Tokens: 1500, Requests: 15},
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	entries, err := c.Costs.History(context.Background(), "2024-01-01", "2024-01-31")
	if err != nil {
		t.Fatal(err)
	}
	if len(entries) != 2 {
		t.Fatalf("got %d", len(entries))
	}
}

func TestCosts_History_NoDates(t *testing.T) {
	var gotPath string
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		gotPath = r.URL.String()
		jsonOK(w, []edgequake.CostEntry{})
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, _ = c.Costs.History(context.Background(), "", "")
	if strings.Contains(gotPath, "start_date=") || strings.Contains(gotPath, "end_date=") {
		t.Fatalf("should not include date params for empty values, got %s", gotPath)
	}
}

func TestCosts_Budget(t *testing.T) {
	budget := 100.0
	remaining := 75.0
	srv := mockServer(t, 200, edgequake.BudgetInfo{
		MonthlyBudgetUSD: &budget,
		CurrentSpendUSD:  25.0,
		RemainingUSD:     &remaining,
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	b, err := c.Costs.Budget(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if b.CurrentSpendUSD != 25.0 {
		t.Fatalf("got %f", b.CurrentSpendUSD)
	}
	if *b.MonthlyBudgetUSD != 100.0 {
		t.Fatalf("got %f", *b.MonthlyBudgetUSD)
	}
}

// ── Model Service ────────────────────────────────────────────────────────────

func TestModels_ProviderHealth(t *testing.T) {
	srv := mockServer(t, 200, []edgequake.ProviderHealthInfo{
		{Name: "openai", Enabled: true, Models: []edgequake.ModelInfo{{Name: "gpt-5-nano", IsAvailable: true}}},
		{Name: "ollama", Enabled: false},
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	health, err := c.Models.ProviderHealth(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if len(health) != 2 {
		t.Fatalf("got %d", len(health))
	}
	if health[0].Name != "openai" {
		t.Fatalf("got %s", health[0].Name)
	}
}

// ── Workspace Service ────────────────────────────────────────────────────────

func TestWorkspaces_Get(t *testing.T) {
	srv := mockServer(t, 200, edgequake.WorkspaceInfo{
		ID: "w1", Name: "Default", Slug: "default",
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	ws, err := c.Workspaces.Get(context.Background(), "w1")
	if err != nil {
		t.Fatal(err)
	}
	if ws.Name != "Default" {
		t.Fatalf("got %s", ws.Name)
	}
}

func TestWorkspaces_RebuildEmbeddings(t *testing.T) {
	srv := mockServer(t, 200, edgequake.RebuildResponse{
		Status:  "started",
		Message: "Rebuilding embeddings",
		TrackID: "t-123",
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.Workspaces.RebuildEmbeddings(context.Background(), "w1")
	if err != nil {
		t.Fatal(err)
	}
	if resp.Status != "started" {
		t.Fatalf("got %s", resp.Status)
	}
	if resp.TrackID != "t-123" {
		t.Fatalf("got %s", resp.TrackID)
	}
}

// ── PDF Service ──────────────────────────────────────────────────────────────

func TestPDF_Status(t *testing.T) {
	p := 1.0
	srv := mockServer(t, 200, edgequake.PdfProgressResponse{
		TrackID: "t1", Status: "completed", Progress: &p,
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.PDF.Status(context.Background(), "pdf-1")
	if err != nil {
		t.Fatal(err)
	}
	if resp.Status != "completed" {
		t.Fatalf("got %s", resp.Status)
	}
}

func TestPDF_List(t *testing.T) {
	srv := mockServer(t, 200, []edgequake.Document{
		{ID: "p1", Title: "Paper.pdf"},
		{ID: "p2", Title: "Report.pdf"},
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	pdfs, err := c.PDF.List(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if len(pdfs) != 2 {
		t.Fatalf("got %d", len(pdfs))
	}
}

// ── Error Handling ───────────────────────────────────────────────────────────

func TestError_BadRequest(t *testing.T) {
	srv := mockServer(t, 400, map[string]string{"error": "Bad Request", "message": "invalid"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Documents.Get(context.Background(), "x")
	if !errors.Is(err, edgequake.ErrBadRequest) {
		t.Fatalf("expected ErrBadRequest, got %v", err)
	}
}

func TestError_Forbidden(t *testing.T) {
	srv := mockServer(t, 403, map[string]string{"error": "Forbidden"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Health.Check(context.Background())
	if !errors.Is(err, edgequake.ErrForbidden) {
		t.Fatalf("expected ErrForbidden, got %v", err)
	}
}

func TestError_Conflict(t *testing.T) {
	srv := mockServer(t, 409, map[string]string{"error": "Conflict"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Entities.Create(context.Background(), &edgequake.CreateEntityParams{EntityName: "dup"})
	if !errors.Is(err, edgequake.ErrConflict) {
		t.Fatalf("expected ErrConflict, got %v", err)
	}
}

func TestError_RateLimited(t *testing.T) {
	srv := mockServer(t, 429, map[string]string{"error": "Too Many Requests"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.Health.Check(context.Background())
	if !errors.Is(err, edgequake.ErrRateLimited) {
		t.Fatalf("expected ErrRateLimited, got %v", err)
	}
}

func TestError_APIError_NoMessage(t *testing.T) {
	ae := &edgequake.APIError{StatusCode: 503, ErrorCode: "Service Unavailable"}
	expected := "edgequake: 503 Service Unavailable"
	if ae.Error() != expected {
		t.Fatalf("got %q, want %q", ae.Error(), expected)
	}
}

func TestError_APIError_Is_UnknownTarget(t *testing.T) {
	ae := &edgequake.APIError{StatusCode: 418}
	if errors.Is(ae, edgequake.ErrNotFound) {
		t.Fatal("418 should not match ErrNotFound")
	}
	if errors.Is(ae, edgequake.ErrBadRequest) {
		t.Fatal("418 should not match ErrBadRequest")
	}
}

func TestError_APIError_As(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found", "message": "doc not found", "details": "ID=x"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Documents.Get(context.Background(), "x")
	var apiErr *edgequake.APIError
	if !errors.As(err, &apiErr) {
		t.Fatal("expected APIError")
	}
	if apiErr.StatusCode != 404 {
		t.Fatalf("got %d", apiErr.StatusCode)
	}
	if apiErr.Message != "doc not found" {
		t.Fatalf("got %s", apiErr.Message)
	}
}

// ── Retry Logic ──────────────────────────────────────────────────────────────

func TestRetry_5xx_ThenSuccess(t *testing.T) {
	var calls int32
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		n := atomic.AddInt32(&calls, 1)
		if n <= 2 {
			jsonStatus(w, 500, map[string]string{"error": "Internal Server Error"})
			return
		}
		jsonOK(w, edgequake.HealthResponse{Status: "ok"})
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(3))
	h, err := c.Health.Check(context.Background())
	if err != nil {
		t.Fatalf("expected success after retry, got %v", err)
	}
	if h.Status != "ok" {
		t.Fatalf("got %s", h.Status)
	}
	if atomic.LoadInt32(&calls) < 3 {
		t.Fatalf("expected at least 3 calls, got %d", atomic.LoadInt32(&calls))
	}
}

func TestRetry_429_ThenSuccess(t *testing.T) {
	var calls int32
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		n := atomic.AddInt32(&calls, 1)
		if n == 1 {
			jsonStatus(w, 429, map[string]string{"error": "Too Many Requests"})
			return
		}
		jsonOK(w, edgequake.HealthResponse{Status: "ok"})
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(2))
	h, err := c.Health.Check(context.Background())
	if err != nil {
		t.Fatalf("expected success after retry, got %v", err)
	}
	if h.Status != "ok" {
		t.Fatalf("got %s", h.Status)
	}
}

func TestRetry_MaxExceeded(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(1))
	_, err := c.Health.Check(context.Background())
	if err == nil {
		t.Fatal("expected error after max retries")
	}
	if !errors.Is(err, edgequake.ErrServer) {
		t.Fatalf("expected ErrServer, got %v", err)
	}
}

func TestRetry_NonRetryable_NoRetry(t *testing.T) {
	var calls int32
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		atomic.AddInt32(&calls, 1)
		jsonStatus(w, 404, map[string]string{"error": "Not Found"})
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(3))
	_, err := c.Documents.Get(context.Background(), "missing")
	if err == nil {
		t.Fatal("expected error")
	}
	if atomic.LoadInt32(&calls) != 1 {
		t.Fatalf("non-retryable error should not retry, got %d calls", atomic.LoadInt32(&calls))
	}
}

// ── Context Cancellation ─────────────────────────────────────────────────────

func TestContext_Cancelled(t *testing.T) {
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		time.Sleep(2 * time.Second)
		jsonOK(w, edgequake.HealthResponse{})
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	ctx, cancel := context.WithTimeout(context.Background(), 100*time.Millisecond)
	defer cancel()
	_, err := c.Health.Check(ctx)
	if err == nil {
		t.Fatal("expected error from cancelled context")
	}
}

// ── Request Path Validation ──────────────────────────────────────────────────

func TestRequestPaths(t *testing.T) {
	tests := []struct {
		name   string
		call   func(c *edgequake.Client) error
		method string
		path   string
	}{
		{
			name:   "Health Check",
			call:   func(c *edgequake.Client) error { _, e := c.Health.Check(context.Background()); return e },
			method: "GET",
			path:   "/health",
		},
		{
			name:   "Documents List",
			call:   func(c *edgequake.Client) error { _, e := c.Documents.List(context.Background(), 0, 0); return e },
			method: "GET",
			path:   "/api/v1/documents",
		},
		{
			name:   "Documents Get",
			call:   func(c *edgequake.Client) error { _, e := c.Documents.Get(context.Background(), "abc"); return e },
			method: "GET",
			path:   "/api/v1/documents/abc",
		},
		{
			name:   "Documents Upload",
			call:   func(c *edgequake.Client) error { _, e := c.Documents.UploadText(context.Background(), nil); return e },
			method: "POST",
			path:   "/api/v1/documents",
		},
		{
			name:   "Documents Delete",
			call:   func(c *edgequake.Client) error { return c.Documents.Delete(context.Background(), "abc") },
			method: "DELETE",
			path:   "/api/v1/documents/abc",
		},
		{
			name:   "Graph Get",
			call:   func(c *edgequake.Client) error { _, e := c.Graph.Get(context.Background(), 0); return e },
			method: "GET",
			path:   "/api/v1/graph",
		},
		{
			name:   "Graph Search",
			call:   func(c *edgequake.Client) error { _, e := c.Graph.Search(context.Background(), "q", 0); return e },
			method: "GET",
			path:   "/api/v1/graph/nodes/search",
		},
		{
			name:   "Entities List",
			call:   func(c *edgequake.Client) error { _, e := c.Entities.List(context.Background(), 0, 0, ""); return e },
			method: "GET",
			path:   "/api/v1/graph/entities",
		},
		{
			name: "Entities Create",
			call: func(c *edgequake.Client) error {
				_, e := c.Entities.Create(context.Background(), &edgequake.CreateEntityParams{})
				return e
			},
			method: "POST",
			path:   "/api/v1/graph/entities",
		},
		{
			name: "Entities Merge",
			call: func(c *edgequake.Client) error {
				_, e := c.Entities.Merge(context.Background(), &edgequake.MergeEntitiesParams{})
				return e
			},
			method: "POST",
			path:   "/api/v1/graph/entities/merge",
		},
		{
			name:   "Relationships List",
			call:   func(c *edgequake.Client) error { _, e := c.Relationships.List(context.Background(), 0, 0); return e },
			method: "GET",
			path:   "/api/v1/graph/relationships",
		},
		{
			name: "Query Execute",
			call: func(c *edgequake.Client) error {
				_, e := c.Query.Execute(context.Background(), &edgequake.QueryRequest{})
				return e
			},
			method: "POST",
			path:   "/api/v1/query",
		},
		{
			name: "Chat Completions",
			call: func(c *edgequake.Client) error {
				_, e := c.Chat.Completions(context.Background(), &edgequake.ChatCompletionRequest{})
				return e
			},
			method: "POST",
			path:   "/api/v1/chat/completions",
		},
		{
			name: "Auth Login",
			call: func(c *edgequake.Client) error {
				_, e := c.Auth.Login(context.Background(), &edgequake.LoginParams{})
				return e
			},
			method: "POST",
			path:   "/api/v1/auth/login",
		},
		{
			name:   "Auth Me",
			call:   func(c *edgequake.Client) error { _, e := c.Auth.Me(context.Background()); return e },
			method: "GET",
			path:   "/api/v1/auth/me",
		},
		{
			name: "Auth Refresh",
			call: func(c *edgequake.Client) error {
				_, e := c.Auth.Refresh(context.Background(), &edgequake.RefreshParams{})
				return e
			},
			method: "POST",
			path:   "/api/v1/auth/refresh",
		},
		{
			name:   "Pipeline Status",
			call:   func(c *edgequake.Client) error { _, e := c.Pipeline.Status(context.Background()); return e },
			method: "GET",
			path:   "/api/v1/pipeline/status",
		},
		{
			name:   "Pipeline Metrics",
			call:   func(c *edgequake.Client) error { _, e := c.Pipeline.Metrics(context.Background()); return e },
			method: "GET",
			path:   "/api/v1/pipeline/queue-metrics",
		},
		{
			name:   "Costs Summary",
			call:   func(c *edgequake.Client) error { _, e := c.Costs.Summary(context.Background()); return e },
			method: "GET",
			path:   "/api/v1/costs/summary",
		},
		{
			name:   "Costs Budget",
			call:   func(c *edgequake.Client) error { _, e := c.Costs.Budget(context.Background()); return e },
			method: "GET",
			path:   "/api/v1/costs/budget",
		},
		{
			name:   "Models List",
			call:   func(c *edgequake.Client) error { _, e := c.Models.List(context.Background()); return e },
			method: "GET",
			path:   "/api/v1/models",
		},
		{
			name:   "Models ProviderStatus",
			call:   func(c *edgequake.Client) error { _, e := c.Models.ProviderStatus(context.Background()); return e },
			method: "GET",
			path:   "/api/v1/settings/provider/status",
		},
		{
			name:   "Models ProviderHealth",
			call:   func(c *edgequake.Client) error { _, e := c.Models.ProviderHealth(context.Background()); return e },
			method: "GET",
			path:   "/api/v1/models/health",
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			var gotMethod, gotPath string
			srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
				gotMethod = r.Method
				gotPath = r.URL.Path
				jsonOK(w, map[string]interface{}{})
			})
			defer srv.Close()
			c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
			_ = tt.call(c)
			if gotMethod != tt.method {
				t.Errorf("method: got %s, want %s", gotMethod, tt.method)
			}
			if gotPath != tt.path {
				t.Errorf("path: got %s, want %s", gotPath, tt.path)
			}
		})
	}
}

// ── JSON Serialization ───────────────────────────────────────────────────────

func TestTypes_QueryRequest_JSON(t *testing.T) {
	topK := 10
	stream := false
	qr := edgequake.QueryRequest{
		Query: "test", Mode: "hybrid", TopK: &topK, Stream: &stream,
	}
	b, err := json.Marshal(qr)
	if err != nil {
		t.Fatal(err)
	}
	var decoded edgequake.QueryRequest
	if err := json.Unmarshal(b, &decoded); err != nil {
		t.Fatal(err)
	}
	if decoded.Query != "test" || decoded.Mode != "hybrid" {
		t.Fatalf("got %+v", decoded)
	}
	if *decoded.TopK != 10 {
		t.Fatalf("got %d", *decoded.TopK)
	}
}

func TestTypes_ChatCompletionRequest_JSON(t *testing.T) {
	temp := 0.7
	maxTok := 1000
	// WHY: EdgeQuake uses `message` (singular string), not `messages` array
	req := edgequake.ChatCompletionRequest{
		Message:     "hi",
		Model:       "gpt-5-nano",
		Temperature: &temp,
		MaxTokens:   &maxTok,
	}
	b, err := json.Marshal(req)
	if err != nil {
		t.Fatal(err)
	}
	var decoded edgequake.ChatCompletionRequest
	if err := json.Unmarshal(b, &decoded); err != nil {
		t.Fatal(err)
	}
	if decoded.Model != "gpt-5-nano" {
		t.Fatalf("got %s", decoded.Model)
	}
	if decoded.Message != "hi" {
		t.Fatalf("got %s", decoded.Message)
	}
	if *decoded.Temperature != 0.7 {
		t.Fatalf("got %f", *decoded.Temperature)
	}
}

func TestTypes_Document_JSON(t *testing.T) {
	size := int64(1024)
	count := 5
	doc := edgequake.Document{
		ID:         "d1",
		Title:      "Test",
		Status:     "completed",
		FileSize:   &size,
		ChunkCount: &count,
	}
	b, err := json.Marshal(doc)
	if err != nil {
		t.Fatal(err)
	}
	var decoded edgequake.Document
	if err := json.Unmarshal(b, &decoded); err != nil {
		t.Fatal(err)
	}
	if *decoded.FileSize != 1024 {
		t.Fatalf("got %d", *decoded.FileSize)
	}
}

func TestTypes_TaskInfo_JSON(t *testing.T) {
	errMsg := "something failed"
	task := edgequake.TaskInfo{
		TrackID:      "tk1",
		Status:       "failed",
		ErrorMessage: &errMsg,
		RetryCount:   2,
		MaxRetries:   3,
		Progress: &edgequake.TaskProgress{
			CurrentStep:     "extraction",
			PercentComplete: 50,
			TotalSteps:      4,
		},
	}
	b, err := json.Marshal(task)
	if err != nil {
		t.Fatal(err)
	}
	var decoded edgequake.TaskInfo
	if err := json.Unmarshal(b, &decoded); err != nil {
		t.Fatal(err)
	}
	if *decoded.ErrorMessage != "something failed" {
		t.Fatalf("got %s", *decoded.ErrorMessage)
	}
	if decoded.Progress.PercentComplete != 50 {
		t.Fatalf("got %d", decoded.Progress.PercentComplete)
	}
}

func TestTypes_GraphEdge_JSON(t *testing.T) {
	w := 0.85
	edge := edgequake.GraphEdge{
		Source:     "A",
		Target:     "B",
		EdgeType:   "KNOWS",
		Weight:     &w,
		Properties: map[string]interface{}{"confidence": 0.9},
	}
	b, err := json.Marshal(edge)
	if err != nil {
		t.Fatal(err)
	}
	var decoded edgequake.GraphEdge
	if err := json.Unmarshal(b, &decoded); err != nil {
		t.Fatal(err)
	}
	if *decoded.Weight != 0.85 {
		t.Fatalf("got %f", *decoded.Weight)
	}
}

func TestTypes_ModelInfo_JSON(t *testing.T) {
	model := edgequake.ModelInfo{
		Name:        "gpt-5-nano",
		DisplayName: "GPT-5 Nano",
		ModelType:   "chat",
		IsAvailable: true,
		Capabilities: &edgequake.ModelCapabilities{
			ContextLength:           128000,
			MaxOutputTokens:         4096,
			SupportsVision:          true,
			SupportsFunctionCalling: true,
			SupportsJSONMode:        true,
			SupportsStreaming:       true,
		},
		Cost: &edgequake.ModelCost{
			InputPer1K:  0.001,
			OutputPer1K: 0.003,
		},
	}
	b, err := json.Marshal(model)
	if err != nil {
		t.Fatal(err)
	}
	var decoded edgequake.ModelInfo
	if err := json.Unmarshal(b, &decoded); err != nil {
		t.Fatal(err)
	}
	if decoded.Capabilities.ContextLength != 128000 {
		t.Fatalf("got %d", decoded.Capabilities.ContextLength)
	}
	if decoded.Cost.OutputPer1K != 0.003 {
		t.Fatalf("got %f", decoded.Cost.OutputPer1K)
	}
}

func TestTypes_TenantInfo_JSON(t *testing.T) {
	tenant := edgequake.TenantInfo{
		ID:                        "t1",
		Name:                      "Acme Corp",
		Slug:                      "acme",
		Plan:                      "enterprise",
		IsActive:                  true,
		MaxWorkspaces:             10,
		DefaultLLMModel:           "gpt-5-nano",
		DefaultLLMProvider:        "openai",
		DefaultEmbeddingModel:     "text-embedding-3-small",
		DefaultEmbeddingProvider:  "openai",
		DefaultEmbeddingDimension: 1536,
	}
	b, err := json.Marshal(tenant)
	if err != nil {
		t.Fatal(err)
	}
	var decoded edgequake.TenantInfo
	if err := json.Unmarshal(b, &decoded); err != nil {
		t.Fatal(err)
	}
	if decoded.MaxWorkspaces != 10 {
		t.Fatalf("got %d", decoded.MaxWorkspaces)
	}
	if decoded.DefaultEmbeddingDimension != 1536 {
		t.Fatalf("got %d", decoded.DefaultEmbeddingDimension)
	}
}

// ── UserAgent Header ─────────────────────────────────────────────────────────

func TestClient_UserAgent(t *testing.T) {
	var got string
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		got = r.Header.Get("User-Agent")
		jsonOK(w, edgequake.HealthResponse{Status: "ok"})
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, _ = c.Health.Check(context.Background())
	if got != "edgequake-go/0.1.0" {
		t.Fatalf("expected edgequake-go/0.1.0, got %q", got)
	}
}

func TestClient_AcceptHeader(t *testing.T) {
	var got string
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		got = r.Header.Get("Accept")
		jsonOK(w, edgequake.HealthResponse{Status: "ok"})
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, _ = c.Health.Check(context.Background())
	if got != "application/json" {
		t.Fatalf("expected application/json, got %q", got)
	}
}

func TestClient_ContentTypeOnPost(t *testing.T) {
	var got string
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		got = r.Header.Get("Content-Type")
		jsonOK(w, edgequake.QueryResponse{})
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, _ = c.Query.Execute(context.Background(), &edgequake.QueryRequest{Query: "test"})
	if got != "application/json" {
		t.Fatalf("expected application/json, got %q", got)
	}
}

// ── Edge Cases ───────────────────────────────────────────────────────────────

func TestClient_EmptyResponseBody(t *testing.T) {
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(200)
		// Empty body — decoder should handle this
		_, _ = w.Write([]byte("{}"))
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.Health.Check(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if resp.Status != "" {
		t.Fatalf("expected empty status, got %q", resp.Status)
	}
}

func TestClient_NetworkError(t *testing.T) {
	c := edgequake.NewClient(
		edgequake.WithBaseURL("http://127.0.0.1:1"), // nothing listening
		edgequake.WithMaxRetries(0),
		edgequake.WithTimeout(1*time.Second),
	)
	_, err := c.Health.Check(context.Background())
	if err == nil {
		t.Fatal("expected network error")
	}
}

func TestGraph_Search_QueryEncoding(t *testing.T) {
	var gotQuery string
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		gotQuery = r.URL.Query().Get("q")
		jsonOK(w, edgequake.SearchNodesResponse{})
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, _ = c.Graph.Search(context.Background(), "hello world", 0)
	if gotQuery != "hello world" {
		t.Fatalf("expected 'hello world', got %q", gotQuery)
	}
}

func TestGraph_Get_LimitParam(t *testing.T) {
	var gotPath string
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		gotPath = r.URL.String()
		jsonOK(w, edgequake.GraphResponse{})
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, _ = c.Graph.Get(context.Background(), 50)
	if !strings.Contains(gotPath, "limit=50") {
		t.Fatalf("expected limit=50, got %s", gotPath)
	}
}

func TestGraph_Get_NoLimitParam(t *testing.T) {
	var gotPath string
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		gotPath = r.URL.String()
		jsonOK(w, edgequake.GraphResponse{})
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, _ = c.Graph.Get(context.Background(), 0)
	if strings.Contains(gotPath, "limit=") {
		t.Fatalf("should not include limit for 0, got %s", gotPath)
	}
}

func TestLineage_DepthParam(t *testing.T) {
	var gotPath string
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		gotPath = r.URL.String()
		jsonOK(w, edgequake.LineageGraph{})
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, _ = c.Lineage.ForEntity(context.Background(), "E1", 5)
	if !strings.Contains(gotPath, "depth=5") {
		t.Fatalf("expected depth=5, got %s", gotPath)
	}
}

func TestLineage_NoDepthParam(t *testing.T) {
	var gotPath string
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		gotPath = r.URL.String()
		jsonOK(w, edgequake.LineageGraph{})
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, _ = c.Lineage.ForEntity(context.Background(), "E1", 0)
	if strings.Contains(gotPath, "depth=") {
		t.Fatalf("should not include depth for 0, got %s", gotPath)
	}
}

func TestCosts_History_WithDates(t *testing.T) {
	var gotPath string
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		gotPath = r.URL.String()
		jsonOK(w, []edgequake.CostEntry{})
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, _ = c.Costs.History(context.Background(), "2024-01-01", "2024-12-31")
	if !strings.Contains(gotPath, "start_date=2024-01-01") {
		t.Fatalf("expected start_date, got %s", gotPath)
	}
	if !strings.Contains(gotPath, "end_date=2024-12-31") {
		t.Fatalf("expected end_date, got %s", gotPath)
	}
}

// ── POST Body Validation ─────────────────────────────────────────────────────

func TestDocuments_UploadText_Body(t *testing.T) {
	var gotBody map[string]interface{}
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		b, _ := io.ReadAll(r.Body)
		_ = json.Unmarshal(b, &gotBody)
		jsonStatus(w, 201, edgequake.UploadResponse{ID: "d1"})
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, _ = c.Documents.UploadText(context.Background(), map[string]interface{}{
		"content": "hello",
		"title":   "Test",
	})
	if gotBody["content"] != "hello" {
		t.Fatalf("got content=%v", gotBody["content"])
	}
	if gotBody["title"] != "Test" {
		t.Fatalf("got title=%v", gotBody["title"])
	}
}

func TestEntities_Create_Body(t *testing.T) {
	var gotBody edgequake.CreateEntityParams
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		b, _ := io.ReadAll(r.Body)
		_ = json.Unmarshal(b, &gotBody)
		jsonStatus(w, 201, edgequake.CreateEntityResponse{Status: "created"})
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, _ = c.Entities.Create(context.Background(), &edgequake.CreateEntityParams{
		EntityName:  "NEW_ENTITY",
		EntityType:  "CONCEPT",
		Description: "A test entity",
		SourceID:    "manual",
	})
	if gotBody.EntityName != "NEW_ENTITY" {
		t.Fatalf("got %s", gotBody.EntityName)
	}
	if gotBody.EntityType != "CONCEPT" {
		t.Fatalf("got %s", gotBody.EntityType)
	}
}

func TestConversations_BulkDelete_Body(t *testing.T) {
	var gotBody map[string]interface{}
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		b, _ := io.ReadAll(r.Body)
		_ = json.Unmarshal(b, &gotBody)
		jsonOK(w, edgequake.BulkDeleteResponse{DeletedCount: 2})
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, _ = c.Conversations.BulkDelete(context.Background(), []string{"c1", "c2"})
	ids, ok := gotBody["ids"].([]interface{})
	if !ok || len(ids) != 2 {
		t.Fatalf("expected 2 ids, got %v", gotBody["ids"])
	}
}

// ── Service Initialization ───────────────────────────────────────────────────

func TestClient_AllServicesInitialized(t *testing.T) {
	c := edgequake.NewClient()
	services := []struct {
		name string
		ptr  interface{}
	}{
		{"Health", c.Health}, {"Documents", c.Documents}, {"Graph", c.Graph},
		{"Entities", c.Entities}, {"Relationships", c.Relationships},
		{"Query", c.Query}, {"Chat", c.Chat}, {"Auth", c.Auth},
		{"Users", c.Users}, {"APIKeys", c.APIKeys}, {"Tenants", c.Tenants},
		{"Conversations", c.Conversations}, {"Folders", c.Folders},
		{"Tasks", c.Tasks}, {"Pipeline", c.Pipeline}, {"Costs", c.Costs},
		{"Chunks", c.Chunks}, {"Provenance", c.Provenance},
		{"Lineage", c.Lineage}, {"Models", c.Models},
		{"Workspaces", c.Workspaces}, {"PDF", c.PDF},
	}
	for _, s := range services {
		if s.ptr == nil {
			t.Fatalf("%s is nil", s.name)
		}
	}
}

// ── Error Paths for Services ─────────────────────────────────────────────────
// WHY: Each service method has an error return branch at 75% coverage.
// These tests exercise the error path for every service category.

func TestDocuments_List_Error(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.Documents.List(context.Background(), 1, 10)
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestDocuments_Get_Error(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Documents.Get(context.Background(), "missing")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestDocuments_UploadText_Error(t *testing.T) {
	srv := mockServer(t, 400, map[string]string{"error": "Bad Request"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Documents.UploadText(context.Background(), nil)
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestDocuments_Track_Error(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Documents.Track(context.Background(), "missing")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestDocuments_Scan_Error(t *testing.T) {
	srv := mockServer(t, 400, map[string]string{"error": "Bad Request"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Documents.Scan(context.Background(), &edgequake.ScanRequest{Path: ""})
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestDocuments_DeletionImpact_Error(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Documents.DeletionImpact(context.Background(), "missing")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestGraph_Get_Error(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.Graph.Get(context.Background(), 10)
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestGraph_Search_Error(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.Graph.Search(context.Background(), "q", 10)
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestEntities_List_Error(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.Entities.List(context.Background(), 1, 10, "")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestEntities_Get_Error(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Entities.Get(context.Background(), "MISSING")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestEntities_Create_Error(t *testing.T) {
	srv := mockServer(t, 422, map[string]string{"error": "Validation Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Entities.Create(context.Background(), &edgequake.CreateEntityParams{})
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestEntities_Merge_Error(t *testing.T) {
	srv := mockServer(t, 400, map[string]string{"error": "Bad Request"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Entities.Merge(context.Background(), &edgequake.MergeEntitiesParams{})
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestEntities_Exists_Error(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.Entities.Exists(context.Background(), "X")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestEntities_Neighborhood_Error(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Entities.Neighborhood(context.Background(), "MISSING", 2)
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestRelationships_List_Error(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.Relationships.List(context.Background(), 1, 10)
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestRelationships_Create_Error(t *testing.T) {
	srv := mockServer(t, 422, map[string]string{"error": "Validation Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Relationships.Create(context.Background(), &edgequake.CreateRelationshipParams{})
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestQuery_Execute_Error(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.Query.Execute(context.Background(), &edgequake.QueryRequest{})
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestChat_Completions_Error(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.Chat.Completions(context.Background(), &edgequake.ChatCompletionRequest{})
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestAuth_Login_Error(t *testing.T) {
	srv := mockServer(t, 401, map[string]string{"error": "Unauthorized"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Auth.Login(context.Background(), &edgequake.LoginParams{})
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestAuth_Me_Error(t *testing.T) {
	srv := mockServer(t, 401, map[string]string{"error": "Unauthorized"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Auth.Me(context.Background())
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestAuth_Refresh_Error(t *testing.T) {
	srv := mockServer(t, 401, map[string]string{"error": "Unauthorized"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Auth.Refresh(context.Background(), &edgequake.RefreshParams{})
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestUsers_Create_Error(t *testing.T) {
	srv := mockServer(t, 409, map[string]string{"error": "Conflict"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Users.Create(context.Background(), &edgequake.CreateUserParams{})
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestUsers_Get_Error(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Users.Get(context.Background(), "missing")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestUsers_List_Error(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.Users.List(context.Background())
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestAPIKeys_Create_Error(t *testing.T) {
	srv := mockServer(t, 409, map[string]string{"error": "Conflict"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.APIKeys.Create(context.Background(), "dup")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestAPIKeys_List_Error(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.APIKeys.List(context.Background())
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestTenants_List_Error(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.Tenants.List(context.Background())
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestTenants_Create_Error(t *testing.T) {
	srv := mockServer(t, 409, map[string]string{"error": "Conflict"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Tenants.Create(context.Background(), &edgequake.CreateTenantParams{})
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestConversations_Create_Error(t *testing.T) {
	srv := mockServer(t, 400, map[string]string{"error": "Bad Request"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Conversations.Create(context.Background(), &edgequake.CreateConversationParams{})
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestConversations_List_Error(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.Conversations.List(context.Background())
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestConversations_Get_Error(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Conversations.Get(context.Background(), "missing")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestConversations_CreateMessage_Error(t *testing.T) {
	srv := mockServer(t, 400, map[string]string{"error": "Bad Request"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Conversations.CreateMessage(context.Background(), "c1", &edgequake.CreateMessageParams{})
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestConversations_Share_Error(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Conversations.Share(context.Background(), "missing")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestConversations_BulkDelete_Error(t *testing.T) {
	srv := mockServer(t, 400, map[string]string{"error": "Bad Request"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Conversations.BulkDelete(context.Background(), nil)
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestFolders_Create_Error(t *testing.T) {
	srv := mockServer(t, 400, map[string]string{"error": "Bad Request"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Folders.Create(context.Background(), &edgequake.CreateFolderParams{})
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestFolders_List_Error(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.Folders.List(context.Background())
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestFolders_Get_Error(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Folders.Get(context.Background(), "missing")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestTasks_List_Error(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.Tasks.List(context.Background(), "", 1, 10)
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestTasks_Get_Error(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Tasks.Get(context.Background(), "missing")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestPipeline_Status_Error(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.Pipeline.Status(context.Background())
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestPipeline_Metrics_Error(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.Pipeline.Metrics(context.Background())
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestCosts_Summary_Error(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.Costs.Summary(context.Background())
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestCosts_History_Error(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.Costs.History(context.Background(), "", "")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestCosts_Budget_Error(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.Costs.Budget(context.Background())
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestChunks_Get_Error(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Chunks.Get(context.Background(), "missing")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestProvenance_ForEntity_Error(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Provenance.ForEntity(context.Background(), "MISSING")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestLineage_ForEntity_Error(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Lineage.ForEntity(context.Background(), "MISSING", 2)
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestModels_List_Error(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.Models.List(context.Background())
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestModels_ProviderStatus_Error(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.Models.ProviderStatus(context.Background())
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestModels_ProviderHealth_Error(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.Models.ProviderHealth(context.Background())
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestWorkspaces_ListForTenant_Error(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.Workspaces.ListForTenant(context.Background(), "t1")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestWorkspaces_CreateForTenant_Error(t *testing.T) {
	srv := mockServer(t, 400, map[string]string{"error": "Bad Request"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Workspaces.CreateForTenant(context.Background(), "t1", &edgequake.CreateWorkspaceParams{})
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestWorkspaces_Get_Error(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Workspaces.Get(context.Background(), "missing")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestWorkspaces_Stats_Error(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Workspaces.Stats(context.Background(), "missing")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestWorkspaces_RebuildEmbeddings_Error(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Workspaces.RebuildEmbeddings(context.Background(), "missing")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestPDF_Progress_Error(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.PDF.Progress(context.Background(), "missing")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestPDF_Content_Error(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.PDF.Content(context.Background(), "missing")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestPDF_Status_Error(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.PDF.Status(context.Background(), "missing")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestPDF_List_Error(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.PDF.List(context.Background())
	if err == nil {
		t.Fatal("expected error")
	}
}

// ── Retry with POST body (GetBody) ──────────────────────────────────────────

func TestRetry_PostBody_Resent(t *testing.T) {
	var calls int32
	var bodies []string
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		b, _ := io.ReadAll(r.Body)
		bodies = append(bodies, string(b))
		n := atomic.AddInt32(&calls, 1)
		if n == 1 {
			jsonStatus(w, 500, map[string]string{"error": "Internal Server Error"})
			return
		}
		jsonStatus(w, 200, edgequake.QueryResponse{Answer: "ok"})
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(2))
	resp, err := c.Query.Execute(context.Background(), &edgequake.QueryRequest{Query: "test"})
	if err != nil {
		t.Fatalf("expected success after retry, got %v", err)
	}
	if resp.Answer != "ok" {
		t.Fatalf("got %s", resp.Answer)
	}
	if len(bodies) < 2 {
		t.Fatalf("expected at least 2 request bodies, got %d", len(bodies))
	}
}

// ── Delete/NoContent error paths ────────────────────────────────────────────

func TestDocuments_Delete_Error(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	err := c.Documents.Delete(context.Background(), "missing")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestDocuments_DeleteAll_Error(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	err := c.Documents.DeleteAll(context.Background())
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestEntities_Delete_Error(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	err := c.Entities.Delete(context.Background(), "E1")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestConversations_Delete_Error(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	err := c.Conversations.Delete(context.Background(), "missing")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestFolders_Delete_Error(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	err := c.Folders.Delete(context.Background(), "missing")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestAPIKeys_Revoke_Error(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	err := c.APIKeys.Revoke(context.Background(), "missing")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestTasks_Cancel_Error(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	err := c.Tasks.Cancel(context.Background(), "missing")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestConversations_Pin_Error(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	err := c.Conversations.Pin(context.Background(), "missing")
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestConversations_Unpin_Error(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	err := c.Conversations.Unpin(context.Background(), "missing")
	if err == nil {
		t.Fatal("expected error")
	}
}

// ── Lineage & Metadata Model Tests ──────────────────────────────────
// WHY: The improve-lineage mission requires source_id, metadata,
// and provenance fields to be properly tested across all SDKs.

func TestEntity_SourceID_Field(t *testing.T) {
	e := edgequake.Entity{SourceID: "doc-123", EntityName: "ALICE"}
	if e.SourceID != "doc-123" {
		t.Fatalf("expected doc-123, got %s", e.SourceID)
	}
}

func TestEntity_Metadata_Field(t *testing.T) {
	e := edgequake.Entity{
		EntityName: "BOB",
		Metadata:   map[string]interface{}{"key": "value"},
	}
	if e.Metadata == nil {
		t.Fatal("expected metadata to be non-nil")
	}
}

func TestEntity_Timestamps(t *testing.T) {
	e := edgequake.Entity{
		EntityName: "EVE",
		CreatedAt:  "2025-01-01T00:00:00Z",
		UpdatedAt:  "2025-01-02T00:00:00Z",
	}
	if e.CreatedAt == "" || e.UpdatedAt == "" {
		t.Fatal("timestamps should not be empty")
	}
}

func TestCreateEntityParams_SourceID(t *testing.T) {
	p := edgequake.CreateEntityParams{
		EntityName:  "ALICE",
		EntityType:  "person",
		Description: "A researcher",
		SourceID:    "doc-456",
	}
	if p.SourceID != "doc-456" {
		t.Fatalf("expected doc-456, got %s", p.SourceID)
	}
}

func TestCreateEntityParams_Metadata(t *testing.T) {
	p := edgequake.CreateEntityParams{
		EntityName: "META",
		SourceID:   "src-1",
		Metadata:   map[string]interface{}{"confidence": 0.95},
	}
	if p.Metadata == nil {
		t.Fatal("expected metadata")
	}
}

func TestProvenanceRecord_Fields(t *testing.T) {
	conf := 0.92
	pr := edgequake.ProvenanceRecord{
		EntityID:         "ent-1",
		EntityName:       "ALICE",
		DocumentID:       "doc-1",
		ChunkID:          "chunk-7",
		ExtractionMethod: "llm",
		Confidence:       &conf,
	}
	if pr.DocumentID != "doc-1" {
		t.Fatalf("expected doc-1, got %s", pr.DocumentID)
	}
	if *pr.Confidence != 0.92 {
		t.Fatalf("expected 0.92, got %f", *pr.Confidence)
	}
}

func TestLineageGraph_Structure(t *testing.T) {
	lg := edgequake.LineageGraph{
		Nodes: []edgequake.LineageNode{
			{ID: "n1", Name: "ALICE", NodeType: "person"},
			{ID: "n2", Name: "BOB", NodeType: "person"},
		},
		Edges: []edgequake.LineageEdge{
			{Source: "n1", Target: "n2", Relationship: "KNOWS"},
		},
	}
	if len(lg.Nodes) != 2 {
		t.Fatalf("expected 2 nodes, got %d", len(lg.Nodes))
	}
	if lg.Edges[0].Relationship != "KNOWS" {
		t.Fatalf("expected KNOWS, got %s", lg.Edges[0].Relationship)
	}
}

func TestLineageNode_Fields(t *testing.T) {
	n := edgequake.LineageNode{ID: "n1", Name: "ALICE", NodeType: "person"}
	if n.NodeType != "person" {
		t.Fatalf("expected person, got %s", n.NodeType)
	}
}

func TestLineageEdge_Fields(t *testing.T) {
	e := edgequake.LineageEdge{Source: "A", Target: "B", Relationship: "COLLAB"}
	if e.Source != "A" || e.Target != "B" {
		t.Fatal("source/target mismatch")
	}
}

func TestEntity_JSON_SourceID_Roundtrip(t *testing.T) {
	e := edgequake.Entity{
		EntityName: "ALICE",
		SourceID:   "doc-rt-1",
		Metadata:   map[string]interface{}{"origin": "test"},
	}
	data, err := json.Marshal(e)
	if err != nil {
		t.Fatalf("marshal failed: %v", err)
	}
	var e2 edgequake.Entity
	if err := json.Unmarshal(data, &e2); err != nil {
		t.Fatalf("unmarshal failed: %v", err)
	}
	if e2.SourceID != "doc-rt-1" {
		t.Fatalf("expected doc-rt-1, got %s", e2.SourceID)
	}
}

func TestProvenanceRecord_JSON_Roundtrip(t *testing.T) {
	conf := 0.88
	pr := edgequake.ProvenanceRecord{
		EntityName: "BOB",
		DocumentID: "doc-prov-1",
		Confidence: &conf,
	}
	data, err := json.Marshal(pr)
	if err != nil {
		t.Fatalf("marshal failed: %v", err)
	}
	var pr2 edgequake.ProvenanceRecord
	if err := json.Unmarshal(data, &pr2); err != nil {
		t.Fatalf("unmarshal failed: %v", err)
	}
	if pr2.DocumentID != "doc-prov-1" {
		t.Fatalf("expected doc-prov-1, got %s", pr2.DocumentID)
	}
}

func TestCreateEntityParams_JSON_Roundtrip(t *testing.T) {
	p := edgequake.CreateEntityParams{
		EntityName:  "TEST",
		EntityType:  "concept",
		Description: "A test entity",
		SourceID:    "src-json-1",
		Metadata:    map[string]interface{}{"key": "val"},
	}
	data, err := json.Marshal(p)
	if err != nil {
		t.Fatalf("marshal failed: %v", err)
	}
	if !strings.Contains(string(data), "src-json-1") {
		t.Fatal("JSON should contain source_id")
	}
	if !strings.Contains(string(data), "key") {
		t.Fatal("JSON should contain metadata")
	}
}

func TestMergeEntitiesParams_JSON(t *testing.T) {
	p := edgequake.MergeEntitiesParams{
		SourceEntity: "ALICE_1",
		TargetEntity: "ALICE_2",
	}
	data, err := json.Marshal(p)
	if err != nil {
		t.Fatalf("marshal failed: %v", err)
	}
	if !strings.Contains(string(data), "ALICE_1") || !strings.Contains(string(data), "ALICE_2") {
		t.Fatal("JSON should contain both entities")
	}
}

func TestEntity_Create_SendsSourceID(t *testing.T) {
	// WHY: Verify that entity creation includes source_id in request body
	var capturedBody string
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		body, _ := io.ReadAll(r.Body)
		capturedBody = string(body)
		w.WriteHeader(200)
		json.NewEncoder(w).Encode(map[string]string{"status": "success"})
	}))
	defer srv.Close()

	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, _ = c.Entities.Create(context.Background(), &edgequake.CreateEntityParams{
		EntityName:  "LINEAGE_TEST",
		EntityType:  "person",
		Description: "Testing lineage",
		SourceID:    "doc-lineage-test",
	})
	if !strings.Contains(capturedBody, "doc-lineage-test") {
		t.Fatalf("request body should contain source_id, got: %s", capturedBody)
	}
}

func TestEntity_Neighborhood_Lineage(t *testing.T) {
	// WHY: Neighborhood is a lineage traversal operation
	expected := map[string]interface{}{
		"center": map[string]interface{}{
			"id": "n1", "entity_name": "ALICE",
		},
		"nodes": []interface{}{},
		"edges": []interface{}{},
		"depth": float64(2),
	}
	srv := mockServer(t, 200, expected)
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.Entities.Neighborhood(context.Background(), "ALICE", 2)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if resp == nil {
		t.Fatal("expected non-nil response")
	}
}

// ── OODA-43: Additional Unique Tests ─────────────────────────────────────────

func TestFolders_Create_WithParentID(t *testing.T) {
	var capturedBody string
	srv := routedServer(t, func(w http.ResponseWriter, r *http.Request) {
		body, _ := io.ReadAll(r.Body)
		capturedBody = string(body)
		jsonStatus(w, 201, edgequake.FolderInfo{ID: "f-child", Name: "Child", ParentID: "f-parent"})
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	folder, err := c.Folders.Create(context.Background(), &edgequake.CreateFolderParams{
		Name:     "Child",
		ParentID: "f-parent",
	})
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(capturedBody, "f-parent") {
		t.Fatalf("expected parent_id in body, got: %s", capturedBody)
	}
	if folder.ParentID != "f-parent" {
		t.Fatalf("expected parent_id f-parent, got %s", folder.ParentID)
	}
}

func TestFolders_Get_WithConversationCount(t *testing.T) {
	srv := mockServer(t, 200, edgequake.FolderInfo{
		ID: "folder-abc", Name: "Projects", ConversationCount: 42,
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	folder, err := c.Folders.Get(context.Background(), "folder-abc")
	if err != nil {
		t.Fatal(err)
	}
	if folder.ConversationCount != 42 {
		t.Fatalf("expected 42 conversations, got %d", folder.ConversationCount)
	}
}

func TestFolders_List_Empty(t *testing.T) {
	srv := mockServer(t, 200, []edgequake.FolderInfo{})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	folders, err := c.Folders.List(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if len(folders) != 0 {
		t.Fatalf("expected empty list, got %d", len(folders))
	}
}

func TestPDF_Progress_WithProgress(t *testing.T) {
	progress := 75.0
	srv := mockServer(t, 200, edgequake.PdfProgressResponse{
		TrackID: "track-xyz", Status: "processing", Progress: &progress,
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.PDF.Progress(context.Background(), "track-xyz")
	if err != nil {
		t.Fatal(err)
	}
	if resp.Progress == nil || *resp.Progress != 75.0 {
		t.Fatalf("expected 75%% progress")
	}
}

func TestPDF_Content_WithMarkdown(t *testing.T) {
	srv := mockServer(t, 200, edgequake.PdfContentResponse{
		ID:       "pdf-xyz",
		Markdown: "# Title\n\nSome content here...",
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	content, err := c.PDF.Content(context.Background(), "pdf-xyz")
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(content.Markdown, "# Title") {
		t.Fatalf("expected markdown with title, got: %s", content.Markdown)
	}
}

func TestPDF_List_Multiple(t *testing.T) {
	srv := mockServer(t, 200, []edgequake.Document{
		{ID: "pdf-1", FileName: "report.pdf", Status: "completed"},
		{ID: "pdf-2", FileName: "invoice.pdf", Status: "processing"},
		{ID: "pdf-3", FileName: "contract.pdf", Status: "completed"},
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	pdfs, err := c.PDF.List(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if len(pdfs) != 3 {
		t.Fatalf("expected 3 pdfs, got %d", len(pdfs))
	}
	// Check statuses
	completed := 0
	for _, pdf := range pdfs {
		if pdf.Status == "completed" {
			completed++
		}
	}
	if completed != 2 {
		t.Fatalf("expected 2 completed, got %d", completed)
	}
}

func TestPDF_Status_Processing(t *testing.T) {
	progress := 30.0
	srv := mockServer(t, 200, edgequake.PdfProgressResponse{
		TrackID: "pdf-proc", Status: "processing", Progress: &progress,
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	status, err := c.PDF.Status(context.Background(), "pdf-proc")
	if err != nil {
		t.Fatal(err)
	}
	if status.Status != "processing" {
		t.Fatalf("expected processing, got %s", status.Status)
	}
}

func TestModels_ProviderHealth_WithModels(t *testing.T) {
	srv := mockServer(t, 200, []edgequake.ProviderHealthInfo{
		{
			Name:    "openai",
			Enabled: true,
			Models: []edgequake.ModelInfo{
				{Name: "gpt-4o", DisplayName: "GPT-4o", Provider: "openai"},
				{Name: "gpt-4o-mini", DisplayName: "GPT-4o Mini", Provider: "openai"},
			},
		},
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	health, err := c.Models.ProviderHealth(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if len(health[0].Models) != 2 {
		t.Fatalf("expected 2 models, got %d", len(health[0].Models))
	}
}

func TestModels_ProviderHealth_Disabled(t *testing.T) {
	srv := mockServer(t, 200, []edgequake.ProviderHealthInfo{
		{Name: "openai", Enabled: true, Priority: 1},
		{Name: "anthropic", Enabled: false, Priority: 3},
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	health, err := c.Models.ProviderHealth(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	disabled := 0
	for _, p := range health {
		if !p.Enabled {
			disabled++
		}
	}
	if disabled != 1 {
		t.Fatalf("expected 1 disabled, got %d", disabled)
	}
}

func TestModels_List_WithProviders(t *testing.T) {
	srv := mockServer(t, 200, edgequake.ProviderCatalog{
		Providers: []edgequake.ProviderInfo{
			{Name: "openai", DisplayName: "OpenAI"},
			{Name: "ollama", DisplayName: "Ollama"},
		},
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	catalog, err := c.Models.List(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if len(catalog.Providers) != 2 {
		t.Fatalf("expected 2 providers, got %d", len(catalog.Providers))
	}
}

func TestWorkspaces_RebuildEmbeddings_Response(t *testing.T) {
	srv := mockServer(t, 200, edgequake.RebuildResponse{
		Status:  "started",
		TrackID: "rebuild-123",
		Message: "Rebuild initiated",
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.Workspaces.RebuildEmbeddings(context.Background(), "ws-1")
	if err != nil {
		t.Fatal(err)
	}
	if resp.Status != "started" {
		t.Fatalf("expected started, got %s", resp.Status)
	}
	if resp.TrackID != "rebuild-123" {
		t.Fatalf("expected track id, got %s", resp.TrackID)
	}
}

func TestWorkspaces_Stats_AllCounts(t *testing.T) {
	srv := mockServer(t, 200, edgequake.WorkspaceStats{
		WorkspaceID:       "ws-xyz",
		DocumentCount:     100,
		EntityCount:       500,
		RelationshipCount: 1200,
		ChunkCount:        2000,
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	stats, err := c.Workspaces.Stats(context.Background(), "ws-xyz")
	if err != nil {
		t.Fatal(err)
	}
	if stats.DocumentCount != 100 {
		t.Fatalf("expected 100 documents, got %d", stats.DocumentCount)
	}
	if stats.EntityCount != 500 {
		t.Fatalf("expected 500 entities, got %d", stats.EntityCount)
	}
	if stats.RelationshipCount != 1200 {
		t.Fatalf("expected 1200 relationships, got %d", stats.RelationshipCount)
	}
}

func TestTenants_Create_WithResponse(t *testing.T) {
	srv := mockServer(t, 201, edgequake.TenantInfo{
		ID:   "tenant-new",
		Name: "New Tenant",
		Slug: "new-tenant",
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	tenant, err := c.Tenants.Create(context.Background(), &edgequake.CreateTenantParams{
		Name: "New Tenant",
		Slug: "new-tenant",
	})
	if err != nil {
		t.Fatal(err)
	}
	if tenant.Name != "New Tenant" {
		t.Fatalf("expected New Tenant, got %s", tenant.Name)
	}
}

func TestTenants_List_Multiple(t *testing.T) {
	srv := mockServer(t, 200, edgequake.TenantListResponse{
		Items: []edgequake.TenantInfo{
			{ID: "t1", Name: "Tenant One"},
			{ID: "t2", Name: "Tenant Two"},
		},
		Total: 2,
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.Tenants.List(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if len(resp.Items) != 2 {
		t.Fatalf("expected 2 tenants, got %d", len(resp.Items))
	}
}

func TestPipeline_Status_WithMetrics(t *testing.T) {
	srv := mockServer(t, 200, edgequake.PipelineStatus{
		Status:         "running",
		ActiveTasks:    5,
		QueuedTasks:    12,
		CompletedTasks: 100,
		FailedTasks:    2,
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	status, err := c.Pipeline.Status(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if status.ActiveTasks != 5 {
		t.Fatalf("expected 5 active tasks, got %d", status.ActiveTasks)
	}
	if status.QueuedTasks != 12 {
		t.Fatalf("expected 12 queued tasks, got %d", status.QueuedTasks)
	}
}

func TestPipeline_Metrics_QueueDepth(t *testing.T) {
	srv := mockServer(t, 200, edgequake.QueueMetrics{
		QueueDepth:        25,
		Processing:        10,
		CompletedLastHour: 150,
		FailedLastHour:    3,
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	metrics, err := c.Pipeline.Metrics(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if metrics.QueueDepth != 25 {
		t.Fatalf("expected 25 queue depth, got %d", metrics.QueueDepth)
	}
	if metrics.Processing != 10 {
		t.Fatalf("expected 10 processing, got %d", metrics.Processing)
	}
}

func TestPipeline_Status_InternalError(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "internal"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.Pipeline.Status(context.Background())
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestPipeline_Metrics_ServiceUnavailable(t *testing.T) {
	srv := mockServer(t, 503, map[string]string{"error": "service unavailable"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.Pipeline.Metrics(context.Background())
	if err == nil {
		t.Fatal("expected error")
	}
}

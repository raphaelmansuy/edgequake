package edgequake_test

import (
	"context"
	"encoding/json"
	"errors"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	edgequake "github.com/edgequake/edgequake-go"
)

func mockServer(t *testing.T, statusCode int, body interface{}) *httptest.Server {
	t.Helper()
	return httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(statusCode)
		if body != nil {
			_ = json.NewEncoder(w).Encode(body)
		}
	}))
}

func TestNewClient_Defaults(t *testing.T) {
	c := edgequake.NewClient()
	if c.BaseURL() != "http://localhost:8080" {
		t.Fatalf("got %s", c.BaseURL())
	}
}

func TestNewClient_WithOptions(t *testing.T) {
	c := edgequake.NewClient(
		edgequake.WithBaseURL("http://example.com"),
		edgequake.WithAPIKey("test-key"),
		edgequake.WithTenantID("t1"),
		edgequake.WithWorkspaceID("w1"),
		edgequake.WithMaxRetries(5),
	)
	if c.BaseURL() != "http://example.com" {
		t.Fatalf("got %s", c.BaseURL())
	}
}

func TestNewClient_CustomHTTPClient(t *testing.T) {
	c := edgequake.NewClient(edgequake.WithHTTPClient(&http.Client{}))
	if c == nil {
		t.Fatal("nil client")
	}
}

func TestHealth_Check(t *testing.T) {
	srv := mockServer(t, 200, edgequake.HealthResponse{Status: "healthy", Version: "0.1.0"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	h, err := c.Health.Check(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if h.Status != "healthy" {
		t.Fatalf("got %s", h.Status)
	}
}

func TestDocuments_List(t *testing.T) {
	srv := mockServer(t, 200, edgequake.ListDocumentsResponse{
		Documents: []edgequake.Document{{ID: "d1", Title: "Test"}},
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.Documents.List(context.Background(), 1, 10)
	if err != nil {
		t.Fatal(err)
	}
	if len(resp.Documents) != 1 {
		t.Fatalf("got %d", len(resp.Documents))
	}
}

func TestDocuments_Get(t *testing.T) {
	srv := mockServer(t, 200, edgequake.Document{ID: "d1", Title: "Doc"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	doc, err := c.Documents.Get(context.Background(), "d1")
	if err != nil {
		t.Fatal(err)
	}
	if doc.ID != "d1" {
		t.Fatalf("got %s", doc.ID)
	}
}

func TestDocuments_Delete(t *testing.T) {
	srv := mockServer(t, 204, nil)
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	if err := c.Documents.Delete(context.Background(), "d1"); err != nil {
		t.Fatal(err)
	}
}

func TestDocuments_UploadText(t *testing.T) {
	srv := mockServer(t, 201, edgequake.UploadResponse{ID: "d1", Status: "processing"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.Documents.UploadText(context.Background(), map[string]interface{}{"content": "hello"})
	if err != nil {
		t.Fatal(err)
	}
	if resp.ID != "d1" {
		t.Fatalf("got %s", resp.ID)
	}
}

func TestDocuments_Track(t *testing.T) {
	srv := mockServer(t, 200, edgequake.TrackStatus{TrackID: "t1", Status: "completed"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	ts, err := c.Documents.Track(context.Background(), "t1")
	if err != nil {
		t.Fatal(err)
	}
	if ts.Status != "completed" {
		t.Fatalf("got %s", ts.Status)
	}
}

func TestDocuments_Scan(t *testing.T) {
	srv := mockServer(t, 200, edgequake.ScanResponse{FilesFound: 5})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.Documents.Scan(context.Background(), &edgequake.ScanRequest{Path: "/data"})
	if err != nil {
		t.Fatal(err)
	}
	if resp.FilesFound != 5 {
		t.Fatalf("got %d", resp.FilesFound)
	}
}

func TestGraph_Get(t *testing.T) {
	srv := mockServer(t, 200, edgequake.GraphResponse{
		Nodes: []edgequake.GraphNode{{ID: "n1", Label: "Node1"}},
		Edges: []edgequake.GraphEdge{{Source: "n1", Target: "n2"}},
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	g, err := c.Graph.Get(context.Background(), 100)
	if err != nil {
		t.Fatal(err)
	}
	if len(g.Nodes) != 1 {
		t.Fatalf("got %d", len(g.Nodes))
	}
}

func TestGraph_Search(t *testing.T) {
	srv := mockServer(t, 200, edgequake.SearchNodesResponse{Nodes: []edgequake.GraphNode{{ID: "n1"}}})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.Graph.Search(context.Background(), "test", 10)
	if err != nil {
		t.Fatal(err)
	}
	if len(resp.Nodes) != 1 {
		t.Fatalf("got %d", len(resp.Nodes))
	}
}

func TestEntities_List(t *testing.T) {
	srv := mockServer(t, 200, edgequake.EntityListResponse{Items: []edgequake.Entity{{EntityName: "ENTITY_A"}}, Total: 1, Page: 1, PageSize: 20, TotalPages: 1})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	ents, err := c.Entities.List(context.Background(), 0, 0, "")
	if err != nil {
		t.Fatal(err)
	}
	if ents.Total != 1 {
		t.Fatalf("got %d", ents.Total)
	}
}

func TestEntities_Create(t *testing.T) {
	srv := mockServer(t, 201, edgequake.CreateEntityResponse{Status: "created", Message: "ok", Entity: &edgequake.Entity{EntityName: "NEW", EntityType: "PERSON"}})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	e, err := c.Entities.Create(context.Background(), &edgequake.CreateEntityParams{EntityName: "NEW", EntityType: "PERSON", Description: "test", SourceID: "manual"})
	if err != nil {
		t.Fatal(err)
	}
	if e.Entity.EntityName != "NEW" {
		t.Fatalf("got %s", e.Entity.EntityName)
	}
}

func TestEntities_Merge(t *testing.T) {
	srv := mockServer(t, 200, edgequake.MergeResponse{MergedCount: 2})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.Entities.Merge(context.Background(), &edgequake.MergeEntitiesParams{SourceEntity: "A", TargetEntity: "B"})
	if err != nil {
		t.Fatal(err)
	}
	if resp.MergedCount != 2 {
		t.Fatalf("got %d", resp.MergedCount)
	}
}

func TestEntities_Delete(t *testing.T) {
	srv := mockServer(t, 204, nil)
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	if err := c.Entities.Delete(context.Background(), "E1"); err != nil {
		t.Fatal(err)
	}
}

func TestEntities_Neighborhood(t *testing.T) {
	srv := mockServer(t, 200, edgequake.NeighborhoodResponse{Nodes: []edgequake.GraphNode{{ID: "n1"}}, Depth: 2})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.Entities.Neighborhood(context.Background(), "E1", 2)
	if err != nil {
		t.Fatal(err)
	}
	if resp.Depth != 2 {
		t.Fatalf("got %d", resp.Depth)
	}
}

func TestRelationships_List(t *testing.T) {
	srv := mockServer(t, 200, edgequake.RelationshipListResponse{Items: []edgequake.Relationship{{Source: "A", Target: "B"}}, Total: 1, Page: 1, PageSize: 20, TotalPages: 1})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	rels, err := c.Relationships.List(context.Background(), 0, 0)
	if err != nil {
		t.Fatal(err)
	}
	if rels.Total != 1 {
		t.Fatalf("got %d", rels.Total)
	}
}

func TestRelationships_Create(t *testing.T) {
	srv := mockServer(t, 201, edgequake.Relationship{Source: "A", Target: "B", RelationshipType: "KNOWS"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	r, err := c.Relationships.Create(context.Background(), &edgequake.CreateRelationshipParams{Source: "A", Target: "B", RelationshipType: "KNOWS"})
	if err != nil {
		t.Fatal(err)
	}
	if r.RelationshipType != "KNOWS" {
		t.Fatalf("got %s", r.RelationshipType)
	}
}

func TestQuery_Execute(t *testing.T) {
	srv := mockServer(t, 200, edgequake.QueryResponse{Answer: "42", Mode: "hybrid"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.Query.Execute(context.Background(), &edgequake.QueryRequest{Query: "What?", Mode: "hybrid"})
	if err != nil {
		t.Fatal(err)
	}
	if resp.Answer != "42" {
		t.Fatalf("got %s", resp.Answer)
	}
}

func TestChat_Completions(t *testing.T) {
	// WHY: EdgeQuake chat API returns {conversation_id, content, sources}, not OpenAI choices
	srv := mockServer(t, 200, edgequake.ChatCompletionResponse{
		ConversationID: "test-conv-id",
		Content:        "Hi! I can help you.",
		Mode:           "hybrid",
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.Chat.Completions(context.Background(), &edgequake.ChatCompletionRequest{
		Message: "Hello",
	})
	if err != nil {
		t.Fatal(err)
	}
	if resp.Content != "Hi! I can help you." {
		t.Fatalf("got %q", resp.Content)
	}
	if resp.ConversationID != "test-conv-id" {
		t.Fatalf("got conversation_id %q", resp.ConversationID)
	}
}

func TestAuth_Login(t *testing.T) {
	srv := mockServer(t, 200, edgequake.TokenResponse{AccessToken: "tok123", TokenType: "Bearer"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	tok, err := c.Auth.Login(context.Background(), &edgequake.LoginParams{Username: "admin", Password: "pass"})
	if err != nil {
		t.Fatal(err)
	}
	if tok.AccessToken != "tok123" {
		t.Fatalf("got %s", tok.AccessToken)
	}
}

func TestAuth_Me(t *testing.T) {
	srv := mockServer(t, 200, edgequake.UserInfo{ID: "u1", Username: "admin"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	u, err := c.Auth.Me(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if u.Username != "admin" {
		t.Fatalf("got %s", u.Username)
	}
}

func TestUsers_Create(t *testing.T) {
	srv := mockServer(t, 201, edgequake.UserInfo{ID: "u2", Username: "new"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	u, err := c.Users.Create(context.Background(), &edgequake.CreateUserParams{Username: "new", Email: "e@e.com", Password: "p"})
	if err != nil {
		t.Fatal(err)
	}
	if u.ID != "u2" {
		t.Fatalf("got %s", u.ID)
	}
}

func TestAPIKeys_Create(t *testing.T) {
	srv := mockServer(t, 201, edgequake.APIKeyResponse{ID: "k1", Key: "secret"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	k, err := c.APIKeys.Create(context.Background(), "my-key")
	if err != nil {
		t.Fatal(err)
	}
	if k.Key != "secret" {
		t.Fatalf("got %s", k.Key)
	}
}

func TestAPIKeys_Revoke(t *testing.T) {
	srv := mockServer(t, 204, nil)
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	if err := c.APIKeys.Revoke(context.Background(), "k1"); err != nil {
		t.Fatal(err)
	}
}

func TestTenants_List(t *testing.T) {
	srv := mockServer(t, 200, edgequake.TenantListResponse{Items: []edgequake.TenantInfo{{ID: "t1", Name: "Main"}}, Total: 1})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	ts, err := c.Tenants.List(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if ts.Total != 1 {
		t.Fatalf("got %d", ts.Total)
	}
}

func TestTenants_Create(t *testing.T) {
	srv := mockServer(t, 201, edgequake.TenantInfo{ID: "t2", Name: "New"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	tenant, err := c.Tenants.Create(context.Background(), &edgequake.CreateTenantParams{Name: "New"})
	if err != nil {
		t.Fatal(err)
	}
	if tenant.Name != "New" {
		t.Fatalf("got %s", tenant.Name)
	}
}

func TestConversations_Create(t *testing.T) {
	srv := mockServer(t, 201, edgequake.ConversationInfo{ID: "c1", Title: "Chat"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	conv, err := c.Conversations.Create(context.Background(), &edgequake.CreateConversationParams{Title: "Chat"})
	if err != nil {
		t.Fatal(err)
	}
	if conv.ID != "c1" {
		t.Fatalf("got %s", conv.ID)
	}
}

func TestConversations_CreateMessage(t *testing.T) {
	srv := mockServer(t, 201, edgequake.Message{ID: "m1", Role: "user", Content: "Hi"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	msg, err := c.Conversations.CreateMessage(context.Background(), "c1", &edgequake.CreateMessageParams{Role: "user", Content: "Hi"})
	if err != nil {
		t.Fatal(err)
	}
	if msg.Content != "Hi" {
		t.Fatalf("got %s", msg.Content)
	}
}

func TestConversations_Share(t *testing.T) {
	srv := mockServer(t, 200, edgequake.ShareLink{ShareID: "s1"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	link, err := c.Conversations.Share(context.Background(), "c1")
	if err != nil {
		t.Fatal(err)
	}
	if link.ShareID != "s1" {
		t.Fatalf("got %s", link.ShareID)
	}
}

func TestConversations_BulkDelete(t *testing.T) {
	srv := mockServer(t, 200, edgequake.BulkDeleteResponse{Affected: 3})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.Conversations.BulkDelete(context.Background(), []string{"c1", "c2", "c3"})
	if err != nil {
		t.Fatal(err)
	}
	if resp.Affected != 3 {
		t.Fatalf("got %d", resp.Affected)
	}
}

func TestFolders_Create(t *testing.T) {
	srv := mockServer(t, 201, edgequake.FolderInfo{ID: "f1", Name: "Folder"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	f, err := c.Folders.Create(context.Background(), &edgequake.CreateFolderParams{Name: "Folder"})
	if err != nil {
		t.Fatal(err)
	}
	if f.Name != "Folder" {
		t.Fatalf("got %s", f.Name)
	}
}

func TestTasks_List(t *testing.T) {
	srv := mockServer(t, 200, edgequake.TaskListResponse{Tasks: []edgequake.TaskInfo{{TrackID: "tk1", Status: "running"}}, Total: 1})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.Tasks.List(context.Background(), "", 0, 0)
	if err != nil {
		t.Fatal(err)
	}
	if resp.Total != 1 {
		t.Fatalf("got %d", resp.Total)
	}
}

func TestTasks_Cancel(t *testing.T) {
	srv := mockServer(t, 204, nil)
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	if err := c.Tasks.Cancel(context.Background(), "tk1"); err != nil {
		t.Fatal(err)
	}
}

func TestPipeline_Status(t *testing.T) {
	srv := mockServer(t, 200, edgequake.PipelineStatus{Status: "running", ActiveTasks: 2})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	ps, err := c.Pipeline.Status(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if ps.ActiveTasks != 2 {
		t.Fatalf("got %d", ps.ActiveTasks)
	}
}

func TestPipeline_Metrics(t *testing.T) {
	srv := mockServer(t, 200, edgequake.QueueMetrics{QueueDepth: 5, Processing: 1})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	m, err := c.Pipeline.Metrics(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if m.QueueDepth != 5 {
		t.Fatalf("got %d", m.QueueDepth)
	}
}

func TestCosts_Summary(t *testing.T) {
	srv := mockServer(t, 200, edgequake.CostSummary{TotalCostUSD: 1.23, TotalTokens: 5000})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	cs, err := c.Costs.Summary(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if cs.TotalCostUSD != 1.23 {
		t.Fatalf("got %f", cs.TotalCostUSD)
	}
}

func TestChunks_Get(t *testing.T) {
	srv := mockServer(t, 200, edgequake.ChunkDetail{ID: "ch1", Content: "Hello"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	chunk, err := c.Chunks.Get(context.Background(), "ch1")
	if err != nil {
		t.Fatal(err)
	}
	if chunk.ID != "ch1" {
		t.Fatalf("got %s", chunk.ID)
	}
}

func TestProvenance_ForEntity(t *testing.T) {
	srv := mockServer(t, 200, []edgequake.ProvenanceRecord{{EntityName: "E1", DocumentID: "d1"}})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	recs, err := c.Provenance.ForEntity(context.Background(), "E1")
	if err != nil {
		t.Fatal(err)
	}
	if len(recs) != 1 {
		t.Fatalf("got %d", len(recs))
	}
}

func TestLineage_ForEntity(t *testing.T) {
	srv := mockServer(t, 200, edgequake.LineageGraph{Nodes: []edgequake.LineageNode{{ID: "l1", Name: "Node"}}})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	g, err := c.Lineage.ForEntity(context.Background(), "E1", 3)
	if err != nil {
		t.Fatal(err)
	}
	if len(g.Nodes) != 1 {
		t.Fatalf("got %d", len(g.Nodes))
	}
}

func TestLineage_ForDocument(t *testing.T) {
	srv := mockServer(t, 200, edgequake.DocumentLineageResponse{
		DocumentID:    "d1",
		Entities:      []edgequake.EntitySummary{{Name: "ALICE", Type: "person"}},
		Relationships: []edgequake.RelationshipSummary{{Source: "ALICE", Target: "BOB"}},
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.Lineage.ForDocument(context.Background(), "d1")
	if err != nil {
		t.Fatal(err)
	}
	if resp.DocumentID != "d1" {
		t.Fatalf("got %s", resp.DocumentID)
	}
	if len(resp.Entities) != 1 {
		t.Fatalf("got %d entities", len(resp.Entities))
	}
	if resp.Entities[0].Name != "ALICE" {
		t.Fatalf("got entity %s", resp.Entities[0].Name)
	}
}

func TestLineage_ForDocumentEmpty(t *testing.T) {
	srv := mockServer(t, 200, edgequake.DocumentLineageResponse{DocumentID: "d2"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.Lineage.ForDocument(context.Background(), "d2")
	if err != nil {
		t.Fatal(err)
	}
	if len(resp.Entities) != 0 {
		t.Fatalf("expected empty entities, got %d", len(resp.Entities))
	}
}

func TestLineage_DocumentFullLineage(t *testing.T) {
	srv := mockServer(t, 200, edgequake.DocumentFullLineageResponse{
		DocumentID:  "d1",
		TotalChunks: 5,
		Chunks:      []edgequake.ChunkDetail{{ID: "c1", Content: "hello"}},
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.Lineage.DocumentFullLineage(context.Background(), "d1")
	if err != nil {
		t.Fatal(err)
	}
	if resp.DocumentID != "d1" {
		t.Fatalf("got %s", resp.DocumentID)
	}
	if resp.TotalChunks != 5 {
		t.Fatalf("got %d chunks", resp.TotalChunks)
	}
	if len(resp.Chunks) != 1 {
		t.Fatalf("got %d chunk details", len(resp.Chunks))
	}
}

func TestLineage_ExportLineageJSON(t *testing.T) {
	rawJSON := `{"document_id":"d1","format":"json"}`
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(200)
		w.Write([]byte(rawJSON))
	}))
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	data, err := c.Lineage.ExportLineage(context.Background(), "d1", "json")
	if err != nil {
		t.Fatal(err)
	}
	if string(data) != rawJSON {
		t.Fatalf("got %s", string(data))
	}
}

func TestLineage_ExportLineageCSV(t *testing.T) {
	csv := "entity_name,entity_type\nALICE,person\n"
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		w.Header().Set("Content-Type", "text/csv")
		w.WriteHeader(200)
		w.Write([]byte(csv))
	}))
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	data, err := c.Lineage.ExportLineage(context.Background(), "d1", "csv")
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(data), "ALICE") {
		t.Fatalf("missing ALICE in export: %s", string(data))
	}
}

func TestChunks_Lineage(t *testing.T) {
	srv := mockServer(t, 200, edgequake.ChunkLineageResponse{
		ChunkID:    "c1",
		DocumentID: "d1",
		Entities:   []edgequake.EntitySummary{{Name: "BOB"}},
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.Chunks.Lineage(context.Background(), "c1")
	if err != nil {
		t.Fatal(err)
	}
	if resp.ChunkID != "c1" {
		t.Fatalf("got %s", resp.ChunkID)
	}
	if resp.DocumentID != "d1" {
		t.Fatalf("got doc %s", resp.DocumentID)
	}
	if len(resp.Entities) != 1 {
		t.Fatalf("got %d entities", len(resp.Entities))
	}
}

func TestLineage_ForEntityError(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "not found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Lineage.ForEntity(context.Background(), "MISSING", 0)
	if err == nil {
		t.Fatal("expected error")
	}
	var apiErr *edgequake.APIError
	if !errors.As(err, &apiErr) {
		t.Fatal("expected APIError")
	}
	if apiErr.StatusCode != 404 {
		t.Fatalf("got status %d", apiErr.StatusCode)
	}
}

func TestModels_List(t *testing.T) {
	srv := mockServer(t, 200, edgequake.ProviderCatalog{Providers: []edgequake.ProviderInfo{{Name: "openai", Models: []edgequake.ModelInfo{{Name: "gpt-5-nano", IsAvailable: true}}}}})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	catalog, err := c.Models.List(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if len(catalog.Providers) != 1 {
		t.Fatalf("got %d providers", len(catalog.Providers))
	}
}

func TestModels_ProviderStatus(t *testing.T) {
	srv := mockServer(t, 200, edgequake.ProviderStatus{CurrentProvider: "openai"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	ps, err := c.Models.ProviderStatus(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if ps.CurrentProvider != "openai" {
		t.Fatalf("got %s", ps.CurrentProvider)
	}
}

func TestWorkspaces_ListForTenant(t *testing.T) {
	srv := mockServer(t, 200, edgequake.WorkspaceListResponse{
		Items: []edgequake.WorkspaceInfo{{ID: "w1", Name: "Default"}},
	})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	ws, err := c.Workspaces.ListForTenant(context.Background(), "t1")
	if err != nil {
		t.Fatal(err)
	}
	if len(ws) != 1 {
		t.Fatalf("got %d", len(ws))
	}
}

func TestWorkspaces_CreateForTenant(t *testing.T) {
	srv := mockServer(t, 201, edgequake.WorkspaceInfo{ID: "w2", Name: "New"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	w, err := c.Workspaces.CreateForTenant(context.Background(), "t1", &edgequake.CreateWorkspaceParams{Name: "New"})
	if err != nil {
		t.Fatal(err)
	}
	if w.Name != "New" {
		t.Fatalf("got %s", w.Name)
	}
}

func TestWorkspaces_Stats(t *testing.T) {
	srv := mockServer(t, 200, edgequake.WorkspaceStats{DocumentCount: 10, EntityCount: 50})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	stats, err := c.Workspaces.Stats(context.Background(), "w1")
	if err != nil {
		t.Fatal(err)
	}
	if stats.DocumentCount != 10 {
		t.Fatalf("got %d", stats.DocumentCount)
	}
}

func TestPDF_Progress(t *testing.T) {
	p := 0.75
	srv := mockServer(t, 200, edgequake.PdfProgressResponse{TrackID: "t1", Status: "processing", Progress: &p})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.PDF.Progress(context.Background(), "d1")
	if err != nil {
		t.Fatal(err)
	}
	if resp.Status != "processing" {
		t.Fatalf("got %s", resp.Status)
	}
}

func TestPDF_Content(t *testing.T) {
	srv := mockServer(t, 200, edgequake.PdfContentResponse{ID: "d1", Markdown: "# Hello"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	resp, err := c.PDF.Content(context.Background(), "d1")
	if err != nil {
		t.Fatal(err)
	}
	if resp.Markdown != "# Hello" {
		t.Fatalf("got %s", resp.Markdown)
	}
}

func TestError_NotFound(t *testing.T) {
	srv := mockServer(t, 404, map[string]string{"error": "Not Found", "message": "not found"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Documents.Get(context.Background(), "missing")
	if !errors.Is(err, edgequake.ErrNotFound) {
		t.Fatalf("expected ErrNotFound, got %v", err)
	}
}

func TestError_Unauthorized(t *testing.T) {
	srv := mockServer(t, 401, map[string]string{"error": "Unauthorized"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Health.Check(context.Background())
	if !errors.Is(err, edgequake.ErrUnauthorized) {
		t.Fatalf("expected ErrUnauthorized, got %v", err)
	}
}

func TestError_Validation(t *testing.T) {
	srv := mockServer(t, 422, map[string]string{"error": "Validation Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL))
	_, err := c.Entities.Create(context.Background(), &edgequake.CreateEntityParams{EntityName: ""})
	if !errors.Is(err, edgequake.ErrValidation) {
		t.Fatalf("expected ErrValidation, got %v", err)
	}
}

func TestError_ServerError(t *testing.T) {
	srv := mockServer(t, 500, map[string]string{"error": "Internal Server Error"})
	defer srv.Close()
	c := edgequake.NewClient(edgequake.WithBaseURL(srv.URL), edgequake.WithMaxRetries(0))
	_, err := c.Health.Check(context.Background())
	if !errors.Is(err, edgequake.ErrServer) {
		t.Fatalf("expected ErrServer, got %v", err)
	}
}

func TestError_APIError_Message(t *testing.T) {
	ae := &edgequake.APIError{StatusCode: 400, ErrorCode: "Bad Request", Message: "missing field"}
	if ae.Error() != "edgequake: 400 Bad Request: missing field" {
		t.Fatalf("got %s", ae.Error())
	}
}

func TestError_IsRetryable(t *testing.T) {
	ae429 := &edgequake.APIError{StatusCode: 429}
	ae500 := &edgequake.APIError{StatusCode: 500}
	ae404 := &edgequake.APIError{StatusCode: 404}
	if !ae429.IsRetryable() {
		t.Fatal("429 should be retryable")
	}
	if !ae500.IsRetryable() {
		t.Fatal("500 should be retryable")
	}
	if ae404.IsRetryable() {
		t.Fatal("404 should not be retryable")
	}
}

func TestClient_AuthHeaders(t *testing.T) {
	var headers http.Header
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		headers = r.Header
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(edgequake.HealthResponse{Status: "ok"})
	}))
	defer srv.Close()
	c := edgequake.NewClient(
		edgequake.WithBaseURL(srv.URL),
		edgequake.WithAPIKey("key-123"),
		edgequake.WithTenantID("t-abc"),
		edgequake.WithWorkspaceID("w-xyz"),
	)
	_, err := c.Health.Check(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if headers.Get("X-API-Key") != "key-123" {
		t.Fatalf("got auth: %s", headers.Get("X-API-Key"))
	}
	if headers.Get("X-Tenant-ID") != "t-abc" {
		t.Fatalf("got tenant: %s", headers.Get("X-Tenant-ID"))
	}
	if headers.Get("X-Workspace-ID") != "w-xyz" {
		t.Fatalf("got workspace: %s", headers.Get("X-Workspace-ID"))
	}
}

func TestTypes_JSONRoundTrip(t *testing.T) {
	entity := edgequake.Entity{Name: "SARAH_CHEN", EntityType: "PERSON", Description: "A researcher"}
	b, err := json.Marshal(entity)
	if err != nil {
		t.Fatal(err)
	}
	var decoded edgequake.Entity
	if err := json.Unmarshal(b, &decoded); err != nil {
		t.Fatal(err)
	}
	if decoded.Name != entity.Name {
		t.Fatalf("got %s", decoded.Name)
	}
}

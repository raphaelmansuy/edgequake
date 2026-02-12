//go:build e2e

package edgequake_test

import (
	"context"
	"os"
	"testing"
	"time"

	edgequake "github.com/edgequake/edgequake-go"
)

func e2eBaseURL() string {
	if u := os.Getenv("EDGEQUAKE_BASE_URL"); u != "" {
		return u
	}
	return "http://localhost:8080"
}

func e2eClient() *edgequake.Client {
	opts := []edgequake.Option{
		edgequake.WithBaseURL(e2eBaseURL()),
		edgequake.WithTimeout(15 * time.Second),
	}
	if key := os.Getenv("EDGEQUAKE_API_KEY"); key != "" {
		opts = append(opts, edgequake.WithAPIKey(key))
	}
	// WHY: Default migration-created tenant/user always available for E2E
	tid := os.Getenv("EDGEQUAKE_TENANT_ID")
	if tid == "" {
		tid = "00000000-0000-0000-0000-000000000002"
	}
	opts = append(opts, edgequake.WithTenantID(tid))

	uid := os.Getenv("EDGEQUAKE_USER_ID")
	if uid == "" {
		uid = "00000000-0000-0000-0000-000000000001"
	}
	opts = append(opts, edgequake.WithUserID(uid))

	if wid := os.Getenv("EDGEQUAKE_WORKSPACE_ID"); wid != "" {
		opts = append(opts, edgequake.WithWorkspaceID(wid))
	}
	return edgequake.NewClient(opts...)
}

func TestE2E_Health(t *testing.T) {
	c := e2eClient()
	ctx := context.Background()
	h, err := c.Health.Check(ctx)
	if err != nil {
		t.Fatalf("Health check failed: %v", err)
	}
	if h.Status != "healthy" {
		t.Fatalf("expected healthy, got %s", h.Status)
	}
	if h.Version == "" {
		t.Fatal("version is empty")
	}
	t.Logf("Health: status=%s version=%s storage=%s llm=%s",
		h.Status, h.Version, h.StorageMode, h.LLMProvider)
}

func TestE2E_Documents_ListAndUpload(t *testing.T) {
	c := e2eClient()
	ctx := context.Background()

	list, err := c.Documents.List(ctx, 1, 10)
	if err != nil {
		t.Fatalf("Documents.List failed: %v", err)
	}
	t.Logf("Documents: %d total", len(list.Documents))

	body := map[string]interface{}{
		"content": "EdgeQuake is an advanced Retrieval-Augmented Generation RAG framework implemented in Rust. " +
			"It uses graph-based knowledge representation for enhanced retrieval and entity extraction " +
			"with deduplication algorithms and multi-provider LLM support for production deployments.",
		"title": "E2E Test Document",
	}
	upload, err := c.Documents.UploadText(ctx, body)
	if err != nil {
		t.Fatalf("Documents.UploadText failed: %v", err)
	}
	if upload.ID == "" {
		t.Fatal("upload response has no document_id")
	}
	t.Logf("Uploaded document: id=%s status=%s", upload.ID, upload.Status)

	doc, err := c.Documents.Get(ctx, upload.ID)
	if err != nil {
		t.Fatalf("Documents.Get(%s) failed: %v", upload.ID, err)
	}
	t.Logf("Document: id=%s title=%s status=%s", doc.ID, doc.Title, doc.Status)

	err = c.Documents.Delete(ctx, upload.ID)
	if err != nil {
		t.Logf("Warning: Documents.Delete(%s) failed: %v", upload.ID, err)
	}
}

func TestE2E_Graph_Get(t *testing.T) {
	c := e2eClient()
	ctx := context.Background()
	g, err := c.Graph.Get(ctx, 10)
	if err != nil {
		t.Fatalf("Graph.Get failed: %v", err)
	}
	t.Logf("Graph: %d nodes, %d edges", len(g.Nodes), len(g.Edges))
}

func TestE2E_Graph_Search(t *testing.T) {
	c := e2eClient()
	ctx := context.Background()
	results, err := c.Graph.Search(ctx, "test", 5)
	if err != nil {
		t.Fatalf("Graph.Search failed: %v", err)
	}
	t.Logf("Search results: %d nodes", len(results.Nodes))
}

func TestE2E_Entities_ListAndCreate(t *testing.T) {
	c := e2eClient()
	ctx := context.Background()

	// WHY: Clean up any leftover entity from previous runs to make test idempotent.
	_ = c.Entities.Delete(ctx, "E2E_TEST_ENTITY")

	resp, err := c.Entities.List(ctx, 1, 10, "")
	if err != nil {
		t.Fatalf("Entities.List failed: %v", err)
	}
	t.Logf("Entities: %d total (page %d)", resp.Total, resp.Page)

	created, err := c.Entities.Create(ctx, &edgequake.CreateEntityParams{
		EntityName:  "E2E_TEST_ENTITY",
		EntityType:  "TEST",
		Description: "Created by Go SDK E2E test",
		SourceID:    "manual_entry",
	})
	if err != nil {
		t.Fatalf("Entities.Create failed: %v", err)
	}
	t.Logf("Created entity: %s status=%s", created.Entity.EntityName, created.Status)

	exists, err := c.Entities.Exists(ctx, "E2E_TEST_ENTITY")
	if err != nil {
		t.Fatalf("Entities.Exists failed: %v", err)
	}
	if !exists.Exists {
		t.Fatal("entity should exist after creation")
	}

	got, err := c.Entities.Get(ctx, "E2E_TEST_ENTITY")
	if err != nil {
		t.Fatalf("Entities.Get failed: %v", err)
	}
	if got.Entity == nil || got.Entity.EntityName != "E2E_TEST_ENTITY" {
		name := ""
		if got.Entity != nil {
			name = got.Entity.EntityName
		}
		t.Fatalf("got name %s", name)
	}

	err = c.Entities.Delete(ctx, "E2E_TEST_ENTITY")
	if err != nil {
		t.Logf("Warning: Entities.Delete failed: %v", err)
	}
}

func TestE2E_Relationships_List(t *testing.T) {
	c := e2eClient()
	ctx := context.Background()
	resp, err := c.Relationships.List(ctx, 1, 10)
	if err != nil {
		t.Fatalf("Relationships.List failed: %v", err)
	}
	t.Logf("Relationships: %d total (page %d)", resp.Total, resp.Page)
}

func TestE2E_Query_Execute(t *testing.T) {
	c := e2eClient()
	ctx := context.Background()
	resp, err := c.Query.Execute(ctx, &edgequake.QueryRequest{
		Query: "What is EdgeQuake?",
		Mode:  "hybrid",
	})
	if err != nil {
		t.Logf("Query.Execute returned error (may be expected): %v", err)
		return
	}
	t.Logf("Query response: answer=%q sources=%d",
		truncStr(resp.Answer, 80), len(resp.Sources))
}

func TestE2E_Chat_Completions(t *testing.T) {
	c := e2eClient()
	ctx := context.Background()
	// WHY: EdgeQuake chat API uses `message` (singular string), not messages array
	resp, err := c.Chat.Completions(ctx, &edgequake.ChatCompletionRequest{
		Message: "Hello, what is EdgeQuake?",
	})
	if err != nil {
		t.Logf("Chat.Completions returned error (may need LLM): %v", err)
		return
	}
	if resp.Content != "" {
		t.Logf("Chat response: %q", truncStr(resp.Content, 80))
	}
	if resp.ConversationID != "" {
		t.Logf("Conversation ID: %s", resp.ConversationID)
	}
}

func TestE2E_Conversations_CRUD(t *testing.T) {
	c := e2eClient()
	ctx := context.Background()

	conv, err := c.Conversations.Create(ctx, &edgequake.CreateConversationParams{
		Title: "Go SDK E2E Test Conversation",
	})
	if err != nil {
		t.Fatalf("Conversations.Create failed: %v", err)
	}
	t.Logf("Created conversation: id=%s title=%s", conv.ID, conv.Title)

	convs, err := c.Conversations.List(ctx)
	if err != nil {
		t.Fatalf("Conversations.List failed: %v", err)
	}
	t.Logf("Conversations: %d total", len(convs))

	detail, err := c.Conversations.Get(ctx, conv.ID)
	if err != nil {
		t.Fatalf("Conversations.Get failed: %v", err)
	}
	t.Logf("Conversation detail: id=%s messages=%d",
		detail.ID, len(detail.Messages))

	msg, err := c.Conversations.CreateMessage(ctx, conv.ID,
		&edgequake.CreateMessageParams{
			Role:    "user",
			Content: "Hello from Go SDK E2E test",
		})
	if err != nil {
		t.Fatalf("Conversations.CreateMessage failed: %v", err)
	}
	t.Logf("Created message: id=%s role=%s", msg.ID, msg.Role)

	err = c.Conversations.Delete(ctx, conv.ID)
	if err != nil {
		t.Logf("Warning: Conversations.Delete failed: %v", err)
	}
}

func TestE2E_Folders_CRUD(t *testing.T) {
	c := e2eClient()
	ctx := context.Background()

	folder, err := c.Folders.Create(ctx, &edgequake.CreateFolderParams{
		Name: "Go SDK E2E Test Folder",
	})
	if err != nil {
		t.Fatalf("Folders.Create failed: %v", err)
	}
	t.Logf("Created folder: id=%s name=%s", folder.ID, folder.Name)

	folders, err := c.Folders.List(ctx)
	if err != nil {
		t.Fatalf("Folders.List failed: %v", err)
	}
	t.Logf("Folders: %d total", len(folders))

	err = c.Folders.Delete(ctx, folder.ID)
	if err != nil {
		t.Logf("Warning: Folders.Delete failed: %v", err)
	}
}

func TestE2E_Tasks_List(t *testing.T) {
	c := e2eClient()
	ctx := context.Background()
	tasks, err := c.Tasks.List(ctx, "", 1, 10)
	if err != nil {
		t.Fatalf("Tasks.List failed: %v", err)
	}
	t.Logf("Tasks: %d total", tasks.Total)
}

func TestE2E_Pipeline_Status(t *testing.T) {
	c := e2eClient()
	ctx := context.Background()
	status, err := c.Pipeline.Status(ctx)
	if err != nil {
		t.Fatalf("Pipeline.Status failed: %v", err)
	}
	t.Logf("Pipeline: status=%s active=%d queued=%d",
		status.Status, status.ActiveTasks, status.QueuedTasks)
}

func TestE2E_Pipeline_Metrics(t *testing.T) {
	c := e2eClient()
	ctx := context.Background()
	metrics, err := c.Pipeline.Metrics(ctx)
	if err != nil {
		t.Fatalf("Pipeline.Metrics failed: %v", err)
	}
	t.Logf("Queue metrics: depth=%d processing=%d",
		metrics.QueueDepth, metrics.Processing)
}

func TestE2E_Costs_Summary(t *testing.T) {
	c := e2eClient()
	ctx := context.Background()
	summary, err := c.Costs.Summary(ctx)
	if err != nil {
		t.Fatalf("Costs.Summary failed: %v", err)
	}
	t.Logf("Costs: total=$%.4f tokens=%d",
		summary.TotalCostUSD, summary.TotalTokens)
}

func TestE2E_Costs_Budget(t *testing.T) {
	c := e2eClient()
	ctx := context.Background()
	budget, err := c.Costs.Budget(ctx)
	if err != nil {
		t.Fatalf("Costs.Budget failed: %v", err)
	}
	t.Logf("Budget: current_spend=$%.4f", budget.CurrentSpendUSD)
}

func TestE2E_Models_List(t *testing.T) {
	c := e2eClient()
	ctx := context.Background()
	catalog, err := c.Models.List(ctx)
	if err != nil {
		t.Fatalf("Models.List failed: %v", err)
	}
	t.Logf("Models: %d providers", len(catalog.Providers))
	for _, p := range catalog.Providers {
		t.Logf("  - %s (%s): %d models",
			p.Name, p.DisplayName, len(p.Models))
	}
}

func TestE2E_Models_ProviderStatus(t *testing.T) {
	c := e2eClient()
	ctx := context.Background()
	status, err := c.Models.ProviderStatus(ctx)
	if err != nil {
		t.Fatalf("Models.ProviderStatus failed: %v", err)
	}
	t.Logf("Provider: %s model=%s status=%s",
		status.CurrentProvider, status.CurrentModel, status.Status)
}

func TestE2E_Models_ProviderHealth(t *testing.T) {
	c := e2eClient()
	ctx := context.Background()
	providers, err := c.Models.ProviderHealth(ctx)
	if err != nil {
		t.Fatalf("Models.ProviderHealth failed: %v", err)
	}
	t.Logf("Provider health: %d providers", len(providers))
}

func TestE2E_Tenants_List(t *testing.T) {
	c := e2eClient()
	ctx := context.Background()
	resp, err := c.Tenants.List(ctx)
	if err != nil {
		t.Fatalf("Tenants.List failed: %v", err)
	}
	t.Logf("Tenants: %d total", resp.Total)
	for _, ten := range resp.Items {
		t.Logf("  - %s (slug=%s plan=%s)", ten.Name, ten.Slug, ten.Plan)
	}
}

func TestE2E_Users_List(t *testing.T) {
	c := e2eClient()
	ctx := context.Background()
	resp, err := c.Users.List(ctx)
	if err != nil {
		t.Fatalf("Users.List failed: %v", err)
	}
	t.Logf("Users: %d total", resp.Total)
}

func TestE2E_APIKeys_List(t *testing.T) {
	c := e2eClient()
	ctx := context.Background()
	resp, err := c.APIKeys.List(ctx)
	if err != nil {
		t.Fatalf("APIKeys.List failed: %v", err)
	}
	t.Logf("API Keys: %d total", resp.Total)
}

func TestE2E_Lineage_ForEntity(t *testing.T) {
	c := e2eClient()
	ctx := context.Background()
	graph, err := c.Lineage.ForEntity(ctx, "EDGEQUAKE", 2)
	if err != nil {
		t.Logf("Lineage.ForEntity returned error (expected): %v", err)
		return
	}
	t.Logf("Lineage: %d nodes, %d edges", len(graph.Nodes), len(graph.Edges))
}

func truncStr(s string, n int) string {
	if len(s) <= n {
		return s
	}
	return s[:n] + "..."
}

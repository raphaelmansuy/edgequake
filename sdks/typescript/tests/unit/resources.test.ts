/**
 * Comprehensive resource tests — verifies every resource method's
 * HTTP method, path, and body against a mock transport.
 *
 * WHY: Resources are thin wrappers over base class helpers.
 * Testing them ensures correct endpoint mapping for all 131 API endpoints.
 */

import { beforeEach, describe, expect, it } from "vitest";
import type { HttpTransport } from "../../src/transport/types.js";
import { createMockTransport } from "../helpers/mock-transport.js";

// Resource imports
import { ApiKeysResource } from "../../src/resources/api-keys.js";
import { AuthResource } from "../../src/resources/auth.js";
import { ChatResource } from "../../src/resources/chat.js";
import { ChunksResource } from "../../src/resources/chunks.js";
import { ConversationsResource } from "../../src/resources/conversations.js";
import { CostsResource } from "../../src/resources/costs.js";
import { DocumentsResource } from "../../src/resources/documents.js";
import { FoldersResource } from "../../src/resources/folders.js";
import { GraphResource } from "../../src/resources/graph.js";
import { LineageResource } from "../../src/resources/lineage.js";
import { ModelsResource } from "../../src/resources/models.js";
import { OllamaResource } from "../../src/resources/ollama.js";
import { PipelineResource } from "../../src/resources/pipeline.js";
import { ProvenanceResource } from "../../src/resources/provenance.js";
import { QueryResource } from "../../src/resources/query.js";
import { SettingsResource } from "../../src/resources/settings.js";
import { SharedResource } from "../../src/resources/shared.js";
import { TasksResource } from "../../src/resources/tasks.js";
import { TenantsResource } from "../../src/resources/tenants.js";
import { UsersResource } from "../../src/resources/users.js";
import { WorkspacesResource } from "../../src/resources/workspaces.js";

// ─────────────────────── Auth ───────────────────────

describe("AuthResource", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let auth: AuthResource;

  beforeEach(() => {
    mock = createMockTransport({
      "POST /api/v1/auth/login": {
        body: { access_token: "t", refresh_token: "r" },
      },
      "POST /api/v1/auth/refresh": { body: { access_token: "new-t" } },
      "POST /api/v1/auth/logout": { body: {} },
      "GET /api/v1/auth/me": {
        body: {
          user: {
            user_id: "u1",
            username: "me",
            email: "me@test.com",
            role: "user",
          },
        },
      },
    });
    auth = new AuthResource(mock as unknown as HttpTransport);
  });

  it("login → POST /api/v1/auth/login", async () => {
    const res = await auth.login({ username: "u", password: "p" });
    expect(mock.lastRequest?.method).toBe("POST");
    expect(mock.lastRequest?.path).toBe("/api/v1/auth/login");
    expect(res.access_token).toBe("t");
  });

  it("refresh → POST /api/v1/auth/refresh", async () => {
    await auth.refresh({ refresh_token: "old" });
    expect(mock.lastRequest?.method).toBe("POST");
    expect(mock.lastRequest?.path).toBe("/api/v1/auth/refresh");
  });

  it("logout → POST /api/v1/auth/logout", async () => {
    await auth.logout();
    expect(mock.lastRequest?.method).toBe("POST");
    expect(mock.lastRequest?.path).toBe("/api/v1/auth/logout");
  });

  it("me → GET /api/v1/auth/me", async () => {
    const res = await auth.me();
    expect(mock.lastRequest?.method).toBe("GET");
    expect(res.user.username).toBe("me");
  });
});

// ─────────────────────── Users ───────────────────────

describe("UsersResource", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let users: UsersResource;

  beforeEach(() => {
    mock = createMockTransport({
      "POST /api/v1/users": {
        body: {
          user: {
            user_id: "u1",
            username: "new",
            email: "new@test.com",
            role: "user",
          },
          created_at: "2025-01-01",
        },
      },
      "GET /api/v1/users": {
        body: {
          users: [{ user_id: "u1" }],
          total: 1,
          page: 1,
          page_size: 20,
          total_pages: 1,
        },
      },
      "GET /api/v1/users/u1": { body: { user_id: "u1" } },
      "DELETE /api/v1/users/u1": { body: {} },
    });
    users = new UsersResource(mock as unknown as HttpTransport);
  });

  it("create → POST /api/v1/users", async () => {
    const res = await users.create({
      username: "new",
      email: "new@test.com",
      password: "pw",
      role: "user",
    });
    expect(mock.lastRequest?.method).toBe("POST");
    expect(mock.lastRequest?.path).toBe("/api/v1/users");
    expect(res.user.user_id).toBe("u1");
  });

  it("list → GET /api/v1/users", async () => {
    const res = await users.list();
    expect(mock.lastRequest?.method).toBe("GET");
    expect(res.users).toHaveLength(1);
  });

  it("get → GET /api/v1/users/:id", async () => {
    await users.get("u1");
    expect(mock.lastRequest?.path).toBe("/api/v1/users/u1");
  });

  it("delete → DELETE /api/v1/users/:id", async () => {
    await users.delete("u1");
    expect(mock.lastRequest?.method).toBe("DELETE");
    expect(mock.lastRequest?.path).toBe("/api/v1/users/u1");
  });
});

// ─────────────────────── API Keys ───────────────────────

describe("ApiKeysResource", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let apiKeys: ApiKeysResource;

  beforeEach(() => {
    mock = createMockTransport({
      "POST /api/v1/api-keys": {
        body: {
          key_id: "k1",
          api_key: "eq-key",
          prefix: "eq_",
          scopes: ["read"],
          created_at: "2025-01-01",
        },
      },
      "GET /api/v1/api-keys": {
        body: {
          keys: [{ key_id: "k1" }],
          total: 1,
          page: 1,
          page_size: 20,
          total_pages: 1,
        },
      },
      "DELETE /api/v1/api-keys/k1": {
        body: { key_id: "k1", message: "Key revoked" },
      },
    });
    apiKeys = new ApiKeysResource(mock as unknown as HttpTransport);
  });

  it("create → POST /api/v1/api-keys", async () => {
    const res = await apiKeys.create({ name: "test" });
    expect(mock.lastRequest?.method).toBe("POST");
    expect(res.api_key).toBe("eq-key");
  });

  it("list → GET /api/v1/api-keys", async () => {
    const res = await apiKeys.list();
    expect(res.keys).toHaveLength(1);
  });

  it("revoke → DELETE /api/v1/api-keys/:id", async () => {
    const res = await apiKeys.revoke("k1");
    expect(mock.lastRequest?.method).toBe("DELETE");
    expect(mock.lastRequest?.path).toBe("/api/v1/api-keys/k1");
    expect(res.message).toBe("Key revoked");
  });
});

// ─────────────────────── Documents ───────────────────────

describe("DocumentsResource", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let docs: DocumentsResource;

  beforeEach(() => {
    mock = createMockTransport({
      "POST /api/v1/documents": {
        body: { document_id: "d1", status: "processing" },
      },
      "POST /api/v1/documents/upload": { body: { document_id: "d2" } },
      "POST /api/v1/documents/upload/batch": { body: { results: [] } },
      "GET /api/v1/documents": {
        body: {
          documents: [{ id: "d1", chunk_count: 3 }],
          total: 1,
          page: 1,
          page_size: 20,
          total_pages: 1,
          has_more: false,
          status_counts: {
            pending: 0,
            processing: 0,
            completed: 1,
            partial_failure: 0,
            failed: 0,
            cancelled: 0,
          },
        },
      },
      "GET /api/v1/documents/d1": { body: { id: "d1", title: "test" } },
      "DELETE /api/v1/documents/d1": { body: {} },
      "DELETE /api/v1/documents": { body: {} },
      "GET /api/v1/documents/track/t1": { body: { status: "completed" } },
      "GET /api/v1/documents/d1/deletion-impact": { body: { entities: 5 } },
      "POST /api/v1/documents/scan": { body: { found: 3 } },
      "POST /api/v1/documents/reprocess": { body: { reprocessed: 2 } },
      "POST /api/v1/documents/recover-stuck": { body: { recovered: 1 } },
      "POST /api/v1/documents/d1/retry-chunks": { body: { retried: 3 } },
      "GET /api/v1/documents/d1/failed-chunks": { body: [{ id: "c1" }] },
    });
    docs = new DocumentsResource(mock as unknown as HttpTransport);
  });

  it("upload → POST /api/v1/documents", async () => {
    const res = await docs.upload({ content: "text", title: "test" });
    expect(res.document_id).toBe("d1");
  });

  it("uploadFile → calls transport.upload", async () => {
    const file = new Blob(["hello"]);
    await docs.uploadFile(file);
    expect(mock.lastRequest?.path).toBe("/api/v1/documents/upload");
  });

  it("uploadBatch → calls transport.uploadBatch", async () => {
    await docs.uploadBatch([new Blob(["a"]), new Blob(["b"])]);
    expect(mock.lastRequest?.path).toBe("/api/v1/documents/upload/batch");
  });

  it("list → returns ListDocumentsResponse", async () => {
    const res = await docs.list();
    expect(res.documents).toHaveLength(1);
    expect(res.total).toBe(1);
    expect(res.has_more).toBe(false);
    expect(res.status_counts.completed).toBe(1);
  });

  it("get → GET /api/v1/documents/:id", async () => {
    const doc = await docs.get("d1");
    expect(doc.title).toBe("test");
  });

  it("delete → DELETE /api/v1/documents/:id", async () => {
    await docs.delete("d1");
    expect(mock.lastRequest?.method).toBe("DELETE");
  });

  it("deleteAll → DELETE /api/v1/documents", async () => {
    await docs.deleteAll();
    expect(mock.lastRequest?.method).toBe("DELETE");
    expect(mock.lastRequest?.path).toBe("/api/v1/documents");
  });

  it("getTrackStatus → GET /api/v1/documents/track/:id", async () => {
    const status = await docs.getTrackStatus("t1");
    expect(status.status).toBe("completed");
  });

  it("analyzeDeletionImpact → GET .../deletion-impact", async () => {
    const impact = await docs.analyzeDeletionImpact("d1");
    expect(impact.entities).toBe(5);
  });

  it("scan → POST /api/v1/documents/scan", async () => {
    await docs.scan({ path: "/data" });
    expect(mock.lastRequest?.path).toBe("/api/v1/documents/scan");
  });

  it("reprocessFailed → POST .../reprocess", async () => {
    await docs.reprocessFailed();
    expect(mock.lastRequest?.path).toBe("/api/v1/documents/reprocess");
  });

  it("recoverStuck → POST .../recover-stuck", async () => {
    await docs.recoverStuck();
    expect(mock.lastRequest?.path).toBe("/api/v1/documents/recover-stuck");
  });

  it("retryFailedChunks → POST .../retry-chunks", async () => {
    await docs.retryFailedChunks("d1");
    expect(mock.lastRequest?.path).toBe("/api/v1/documents/d1/retry-chunks");
  });

  it("listFailedChunks → GET .../failed-chunks", async () => {
    const chunks = await docs.listFailedChunks("d1");
    expect(chunks).toHaveLength(1);
  });
});

// ─────────────────────── Documents.PDF ───────────────────────

describe("DocumentsResource.pdf", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let docs: DocumentsResource;

  beforeEach(() => {
    mock = createMockTransport({
      "POST /api/v1/documents/pdf": { body: { pdf_id: "p1" } },
      "GET /api/v1/documents/pdf": { body: [{ id: "p1" }] },
      "GET /api/v1/documents/pdf/p1": { body: { status: "completed" } },
      "GET /api/v1/documents/pdf/p1/content": { body: { markdown: "# Title" } },
      "GET /api/v1/documents/pdf/p1/download": { blob: new Blob(["pdf-data"]) },
      "GET /api/v1/documents/pdf/progress/t1": { body: { progress: 80 } },
      "POST /api/v1/documents/pdf/p1/retry": { body: {} },
      "DELETE /api/v1/documents/pdf/p1/cancel": { body: {} },
      "DELETE /api/v1/documents/pdf/p1": { body: {} },
    });
    docs = new DocumentsResource(mock as unknown as HttpTransport);
  });

  it("pdf.upload → transport.upload", async () => {
    const res = await docs.pdf.upload(new Blob(["pdf"]));
    expect(res.pdf_id).toBe("p1");
  });

  it("pdf.list → GET /api/v1/documents/pdf", async () => {
    const list = await docs.pdf.list();
    expect(list).toHaveLength(1);
  });

  it("pdf.getStatus → GET /api/v1/documents/pdf/:id", async () => {
    const status = await docs.pdf.getStatus("p1");
    expect(status.status).toBe("completed");
  });

  it("pdf.getContent → GET .../content", async () => {
    const content = await docs.pdf.getContent("p1");
    expect(content.markdown).toBe("# Title");
  });

  it("pdf.download → transport.requestBlob", async () => {
    const blob = await docs.pdf.download("p1");
    expect(blob).toBeInstanceOf(Blob);
  });

  it("pdf.getProgress → GET .../progress/:id", async () => {
    const progress = await docs.pdf.getProgress("t1");
    expect(progress.progress).toBe(80);
  });

  it("pdf.retry → POST .../retry", async () => {
    await docs.pdf.retry("p1");
    expect(
      mock.requests.some((r) => r.path === "/api/v1/documents/pdf/p1/retry"),
    ).toBe(true);
  });

  it("pdf.cancel → DELETE .../cancel", async () => {
    await docs.pdf.cancel("p1");
    expect(
      mock.requests.some((r) => r.path === "/api/v1/documents/pdf/p1/cancel"),
    ).toBe(true);
  });

  it("pdf.delete → DELETE .../pdf/:id", async () => {
    await docs.pdf.delete("p1");
    expect(mock.lastRequest?.method).toBe("DELETE");
    expect(mock.lastRequest?.path).toBe("/api/v1/documents/pdf/p1");
  });
});

// ─────────────────────── Query ───────────────────────

describe("QueryResource", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let query: QueryResource;

  beforeEach(() => {
    mock = createMockTransport({
      "POST /api/v1/query": { body: { answer: "42", mode: "hybrid" } },
      "POST /api/v1/query/stream": {
        chunks: ['{"chunk":"Hello "}', '{"chunk":"World"}'],
      },
    });
    query = new QueryResource(mock as unknown as HttpTransport);
  });

  it("execute → POST /api/v1/query", async () => {
    const res = await query.execute({ query: "test", mode: "hybrid" });
    expect(res.answer).toBe("42");
  });

  it("stream → POST /api/v1/query/stream", async () => {
    const events: unknown[] = [];
    for await (const e of query.stream({ query: "test" })) {
      events.push(e);
    }
    expect(events).toHaveLength(2);
  });
});

// ─────────────────────── Chat ───────────────────────

describe("ChatResource", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let chat: ChatResource;

  beforeEach(() => {
    mock = createMockTransport({
      "POST /api/v1/chat/completions": {
        body: { conversation_id: "c1", content: "Hello!", mode: "hybrid" },
      },
      "POST /api/v1/chat/completions/stream": {
        chunks: ['{"type":"token","content":"Hi"}'],
      },
    });
    chat = new ChatResource(mock as unknown as HttpTransport);
  });

  it("completions → POST /api/v1/chat/completions", async () => {
    const res = await chat.completions({
      message: "hello",
    });
    expect(res.conversation_id).toBe("c1");
  });

  it("stream → POST .../completions/stream", async () => {
    const events: unknown[] = [];
    for await (const e of chat.stream({
      message: "hi",
    })) {
      events.push(e);
    }
    expect(events).toHaveLength(1);
  });
});

// ─────────────────────── Conversations ───────────────────────

describe("ConversationsResource", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let conv: ConversationsResource;

  beforeEach(() => {
    mock = createMockTransport({
      "GET /api/v1/conversations": {
        body: {
          items: [{ id: "c1", title: "Test", mode: "hybrid" }],
          pagination: { has_more: false, total: 1 },
        },
      },
      "GET /api/v1/conversations/c1": {
        body: { conversation: { id: "c1", title: "Test" }, messages: [] },
      },
      "POST /api/v1/conversations": { body: { id: "c1" } },
      "PATCH /api/v1/conversations/c1": { body: { id: "c1" } },
      "DELETE /api/v1/conversations/c1": { body: {} },
      "POST /api/v1/conversations/c1/share": { body: { share_id: "s1" } },
      "DELETE /api/v1/conversations/c1/share": { body: {} },
      "POST /api/v1/conversations/import": {
        body: { imported: 1, failed: 0, errors: [] },
      },
      "POST /api/v1/conversations/bulk/delete": { body: { affected: 2 } },
      "POST /api/v1/conversations/bulk/archive": { body: { affected: 2 } },
      "POST /api/v1/conversations/bulk/move": { body: { affected: 2 } },
      // Messages sub-resource
      "GET /api/v1/conversations/c1/messages": {
        body: {
          items: [{ id: "m1" }],
          pagination: { has_more: false },
        },
      },
      "POST /api/v1/conversations/c1/messages": { body: { id: "m1" } },
      "PATCH /api/v1/messages/m1": { body: { id: "m1" } },
      "DELETE /api/v1/messages/m1": { body: {} },
    });
    conv = new ConversationsResource(mock as unknown as HttpTransport);
  });

  it("list → returns PaginatedConversationsResponse", async () => {
    const res = await conv.list();
    expect(res.items).toHaveLength(1);
    expect(res.pagination.has_more).toBe(false);
  });

  it("get → GET /api/v1/conversations/:id", async () => {
    const c = await conv.get("c1");
    expect(c.conversation.title).toBe("Test");
  });

  it("create → POST /api/v1/conversations", async () => {
    await conv.create({ title: "New" });
    expect(mock.lastRequest?.method).toBe("POST");
  });

  it("update → PATCH /api/v1/conversations/:id", async () => {
    await conv.update("c1", { title: "Updated" });
    expect(mock.lastRequest?.method).toBe("PATCH");
  });

  it("delete → DELETE /api/v1/conversations/:id", async () => {
    await conv.delete("c1");
    expect(mock.lastRequest?.method).toBe("DELETE");
  });

  it("share → POST .../share", async () => {
    const res = await conv.share("c1");
    expect(res.share_id).toBe("s1");
  });

  it("unshare → DELETE .../share", async () => {
    await conv.unshare("c1");
    expect(mock.lastRequest?.method).toBe("DELETE");
  });

  it("import → POST .../import", async () => {
    await conv.import({ conversations: [] });
    expect(mock.lastRequest?.path).toBe("/api/v1/conversations/import");
  });

  it("bulkDelete → POST .../bulk/delete", async () => {
    await conv.bulkDelete({ conversation_ids: ["c1", "c2"] });
    expect(mock.lastRequest?.path).toBe("/api/v1/conversations/bulk/delete");
  });

  it("bulkArchive → POST .../bulk/archive", async () => {
    await conv.bulkArchive({ conversation_ids: ["c1"], archive: true });
    expect(mock.lastRequest?.path).toBe("/api/v1/conversations/bulk/archive");
  });

  it("bulkMove → POST .../bulk/move", async () => {
    await conv.bulkMove({ conversation_ids: ["c1"], folder_id: "f1" });
    expect(mock.lastRequest?.path).toBe("/api/v1/conversations/bulk/move");
  });

  // Messages sub-resource
  it("messages.list → GET .../messages (paginated)", async () => {
    const res = await conv.messages.list("c1");
    expect(res.items).toHaveLength(1);
  });

  it("messages.create → POST .../messages", async () => {
    await conv.messages.create("c1", { role: "user", content: "hi" });
    expect(mock.lastRequest?.path).toBe("/api/v1/conversations/c1/messages");
  });

  it("messages.update → PATCH /api/v1/messages/:id", async () => {
    await conv.messages.update("m1", { content: "edited" });
    expect(mock.lastRequest?.path).toBe("/api/v1/messages/m1");
  });

  it("messages.delete → DELETE /api/v1/messages/:id", async () => {
    await conv.messages.delete("m1");
    expect(mock.lastRequest?.path).toBe("/api/v1/messages/m1");
  });
});

// ─────────────────────── Folders ───────────────────────

describe("FoldersResource", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let folders: FoldersResource;

  beforeEach(() => {
    mock = createMockTransport({
      "GET /api/v1/folders": { body: [{ id: "f1" }] },
      "POST /api/v1/folders": { body: { id: "f1" } },
      "PATCH /api/v1/folders/f1": { body: { id: "f1" } },
      "DELETE /api/v1/folders/f1": { body: {} },
    });
    folders = new FoldersResource(mock as unknown as HttpTransport);
  });

  it("list → GET /api/v1/folders", async () => {
    const list = await folders.list();
    expect(list).toHaveLength(1);
  });

  it("create → POST /api/v1/folders", async () => {
    await folders.create({ name: "test" });
    expect(mock.lastRequest?.method).toBe("POST");
  });

  it("update → PATCH /api/v1/folders/:id", async () => {
    await folders.update("f1", { name: "renamed" });
    expect(mock.lastRequest?.method).toBe("PATCH");
  });

  it("delete → DELETE /api/v1/folders/:id", async () => {
    await folders.delete("f1");
    expect(mock.lastRequest?.method).toBe("DELETE");
  });
});

// ─────────────────────── Shared ───────────────────────

describe("SharedResource", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let shared: SharedResource;

  beforeEach(() => {
    mock = createMockTransport({
      "GET /api/v1/shared/s1": { body: { id: "c1", title: "Shared" } },
    });
    shared = new SharedResource(mock as unknown as HttpTransport);
  });

  it("get → GET /api/v1/shared/:id", async () => {
    const res = await shared.get("s1");
    expect(res.title).toBe("Shared");
  });
});

// ─────────────────────── Graph ───────────────────────

describe("GraphResource", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let graph: GraphResource;

  beforeEach(() => {
    mock = createMockTransport({
      "GET /api/v1/graph": { body: { nodes: [], edges: [] } },
      "GET /api/v1/graph/stream": { chunks: ['{"type":"node","data":{}}'] },
      "GET /api/v1/graph/nodes/n1": { body: { id: "n1" } },
      "GET /api/v1/graph/nodes/search": { body: [{ id: "n1" }] },
      "GET /api/v1/graph/labels/search": { body: ["Entity"] },
      "GET /api/v1/graph/labels/popular": { body: ["Entity", "Person"] },
      "POST /api/v1/graph/degrees/batch": { body: { degrees: {} } },
      // Entities
      "GET /api/v1/graph/entities": { body: [{ name: "e1" }] },
      "POST /api/v1/graph/entities": { body: { name: "e1" } },
      "GET /api/v1/graph/entities/e1": { body: { name: "e1" } },
      "GET /api/v1/graph/entities/exists": { body: { exists: true } },
      "PUT /api/v1/graph/entities/e1": { body: { name: "e1" } },
      "DELETE /api/v1/graph/entities/e1": { body: {} },
      "POST /api/v1/graph/entities/merge": { body: { name: "merged" } },
      "GET /api/v1/graph/entities/e1/neighborhood": {
        body: { entity: {}, neighbors: [] },
      },
      // Relationships
      "GET /api/v1/graph/relationships": { body: [{ id: "r1" }] },
      "POST /api/v1/graph/relationships": { body: { id: "r1" } },
      "GET /api/v1/graph/relationships/r1": { body: { id: "r1" } },
      "PUT /api/v1/graph/relationships/r1": { body: { id: "r1" } },
      "DELETE /api/v1/graph/relationships/r1": { body: {} },
    });
    graph = new GraphResource(mock as unknown as HttpTransport);
  });

  it("get → GET /api/v1/graph", async () => {
    const res = await graph.get();
    expect(res.nodes).toBeDefined();
  });

  it("stream → SSE /api/v1/graph/stream", async () => {
    const events: unknown[] = [];
    for await (const e of graph.stream()) {
      events.push(e);
    }
    expect(events).toHaveLength(1);
  });

  it("getNode → GET /api/v1/graph/nodes/:id", async () => {
    const node = await graph.getNode("n1");
    expect(node.id).toBe("n1");
  });

  it("searchNodes → GET /api/v1/graph/nodes/search", async () => {
    await graph.searchNodes({ query: "test" });
    expect(mock.lastRequest?.method).toBe("GET");
  });

  it("searchLabels → GET /api/v1/graph/labels/search", async () => {
    await graph.searchLabels({ query: "Entity" });
    expect(mock.lastRequest?.method).toBe("GET");
  });

  it("getPopularLabels → GET .../labels/popular", async () => {
    const labels = await graph.getPopularLabels();
    expect(labels).toHaveLength(2);
  });

  it("getDegreesBatch → POST .../degrees/batch", async () => {
    await graph.getDegreesBatch({ names: ["e1"] });
    expect(mock.lastRequest?.method).toBe("POST");
  });

  // Entities sub-resource
  it("entities.list → GET /api/v1/graph/entities", async () => {
    const list = await graph.entities.list();
    expect(list).toHaveLength(1);
  });

  it("entities.create → POST /api/v1/graph/entities", async () => {
    await graph.entities.create({
      entity_name: "FOO",
      entity_type: "PERSON",
      description: "Test entity",
      source_id: "manual_entry",
    });
    expect(mock.lastRequest?.method).toBe("POST");
  });

  it("entities.get → GET .../entities/:name", async () => {
    const e = await graph.entities.get("e1");
    expect(e.name).toBe("e1");
  });

  it("entities.exists → GET .../entities/:name (returns boolean)", async () => {
    const exists = await graph.entities.exists("e1");
    expect(exists).toBe(true);
  });

  it("entities.update → PUT .../entities/:name", async () => {
    await graph.entities.update("e1", { description: "updated" });
    expect(mock.lastRequest?.method).toBe("PUT");
  });

  it("entities.delete → DELETE .../entities/:name", async () => {
    await graph.entities.delete("e1");
    expect(mock.lastRequest?.method).toBe("DELETE");
  });

  it("entities.merge → POST .../entities/merge", async () => {
    await graph.entities.merge({ source: "e1", target: "e2" });
    expect(mock.lastRequest?.path).toBe("/api/v1/graph/entities/merge");
  });

  it("entities.neighborhood → GET .../neighborhood", async () => {
    const hood = await graph.entities.neighborhood("e1");
    expect(hood.neighbors).toBeDefined();
  });

  // Relationships sub-resource
  it("relationships.list → GET .../relationships", async () => {
    const list = await graph.relationships.list();
    expect(list).toHaveLength(1);
  });

  it("relationships.create → POST .../relationships", async () => {
    await graph.relationships.create({
      source: "e1",
      target: "e2",
      relationship_type: "KNOWS",
    });
    expect(mock.lastRequest?.method).toBe("POST");
  });

  it("relationships.get → GET .../relationships/:id", async () => {
    const r = await graph.relationships.get("r1");
    expect(r.id).toBe("r1");
  });

  it("relationships.update → PUT .../relationships/:id", async () => {
    await graph.relationships.update("r1", { weight: 0.9 });
    expect(mock.lastRequest?.method).toBe("PUT");
  });

  it("relationships.delete → DELETE .../relationships/:id", async () => {
    await graph.relationships.delete("r1");
    expect(mock.lastRequest?.method).toBe("DELETE");
  });
});

// ─────────────────────── Tenants ───────────────────────

describe("TenantsResource", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let tenants: TenantsResource;

  beforeEach(() => {
    mock = createMockTransport({
      "POST /api/v1/tenants": { body: { id: "t1" } },
      "GET /api/v1/tenants": { body: [{ id: "t1" }] },
      "GET /api/v1/tenants/t1": { body: { id: "t1" } },
      "PUT /api/v1/tenants/t1": { body: { id: "t1" } },
      "DELETE /api/v1/tenants/t1": { body: {} },
      "POST /api/v1/tenants/t1/workspaces": { body: { id: "w1" } },
      "GET /api/v1/tenants/t1/workspaces": { body: [{ id: "w1" }] },
      "GET /api/v1/tenants/t1/workspaces/by-slug/main": { body: { id: "w1" } },
    });
    tenants = new TenantsResource(mock as unknown as HttpTransport);
  });

  it("create → POST /api/v1/tenants", async () => {
    await tenants.create({ name: "Test" });
    expect(mock.lastRequest?.method).toBe("POST");
  });

  it("list → GET /api/v1/tenants", async () => {
    const list = await tenants.list();
    expect(list).toHaveLength(1);
  });

  it("get → GET .../tenants/:id", async () => {
    await tenants.get("t1");
    expect(mock.lastRequest?.path).toBe("/api/v1/tenants/t1");
  });

  it("update → PUT .../tenants/:id", async () => {
    await tenants.update("t1", { name: "Renamed" });
    expect(mock.lastRequest?.method).toBe("PUT");
  });

  it("delete → DELETE .../tenants/:id", async () => {
    await tenants.delete("t1");
    expect(mock.lastRequest?.method).toBe("DELETE");
  });

  it("createWorkspace → POST .../workspaces", async () => {
    await tenants.createWorkspace("t1", { name: "ws1", slug: "ws1" });
    expect(mock.lastRequest?.path).toBe("/api/v1/tenants/t1/workspaces");
  });

  it("listWorkspaces → GET .../workspaces", async () => {
    const list = await tenants.listWorkspaces("t1");
    expect(list).toHaveLength(1);
  });

  it("getWorkspaceBySlug → GET .../by-slug/:slug", async () => {
    await tenants.getWorkspaceBySlug("t1", "main");
    expect(mock.lastRequest?.path).toBe(
      "/api/v1/tenants/t1/workspaces/by-slug/main",
    );
  });
});

// ─────────────────────── Workspaces ───────────────────────

describe("WorkspacesResource", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let workspaces: WorkspacesResource;

  beforeEach(() => {
    mock = createMockTransport({
      "GET /api/v1/workspaces/w1": { body: { id: "w1" } },
      "PUT /api/v1/workspaces/w1": { body: { id: "w1" } },
      "DELETE /api/v1/workspaces/w1": { body: {} },
      "GET /api/v1/workspaces/w1/stats": { body: { documents: 10 } },
      "GET /api/v1/workspaces/w1/metrics-history": { body: { data: [] } },
      "POST /api/v1/workspaces/w1/metrics-snapshot": { body: {} },
      "POST /api/v1/workspaces/w1/rebuild-embeddings": { body: {} },
      "POST /api/v1/workspaces/w1/rebuild-knowledge-graph": { body: {} },
      "POST /api/v1/workspaces/w1/reprocess-documents": { body: {} },
    });
    workspaces = new WorkspacesResource(mock as unknown as HttpTransport);
  });

  it("get → GET .../workspaces/:id", async () => {
    await workspaces.get("w1");
    expect(mock.lastRequest?.path).toBe("/api/v1/workspaces/w1");
  });

  it("update → PUT .../workspaces/:id", async () => {
    await workspaces.update("w1", { name: "Updated" });
    expect(mock.lastRequest?.method).toBe("PUT");
  });

  it("delete → DELETE .../workspaces/:id", async () => {
    await workspaces.delete("w1");
    expect(mock.lastRequest?.method).toBe("DELETE");
  });

  it("stats → GET .../stats", async () => {
    const stats = await workspaces.stats("w1");
    expect(stats.documents).toBe(10);
  });

  it("metricsHistory → GET .../metrics-history", async () => {
    await workspaces.metricsHistory("w1");
    expect(mock.lastRequest?.path).toBe(
      "/api/v1/workspaces/w1/metrics-history",
    );
  });

  it("triggerMetricsSnapshot → POST .../metrics/snapshot", async () => {
    await workspaces.triggerMetricsSnapshot("w1");
    expect(mock.lastRequest?.method).toBe("POST");
  });

  it("rebuildEmbeddings → POST .../rebuild-embeddings", async () => {
    await workspaces.rebuildEmbeddings("w1");
    expect(mock.lastRequest?.path).toBe(
      "/api/v1/workspaces/w1/rebuild-embeddings",
    );
  });

  it("rebuildKnowledgeGraph → POST .../rebuild-knowledge-graph", async () => {
    await workspaces.rebuildKnowledgeGraph("w1");
    expect(mock.lastRequest?.path).toBe(
      "/api/v1/workspaces/w1/rebuild-knowledge-graph",
    );
  });

  it("reprocessDocuments → POST .../reprocess-documents", async () => {
    await workspaces.reprocessDocuments("w1");
    expect(mock.lastRequest?.path).toBe(
      "/api/v1/workspaces/w1/reprocess-documents",
    );
  });
});

// ─────────────────────── Tasks ───────────────────────

describe("TasksResource", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let tasks: TasksResource;

  beforeEach(() => {
    mock = createMockTransport({
      "GET /api/v1/tasks/t1": { body: { id: "t1", status: "running" } },
      "GET /api/v1/tasks": { body: [{ id: "t1" }] },
      "POST /api/v1/tasks/t1/cancel": { body: {} },
      "POST /api/v1/tasks/t1/retry": { body: { id: "t1" } },
    });
    tasks = new TasksResource(mock as unknown as HttpTransport);
  });

  it("get → GET /api/v1/tasks/:id", async () => {
    const task = await tasks.get("t1");
    expect(task.status).toBe("running");
  });

  it("list → GET /api/v1/tasks", async () => {
    const list = await tasks.list();
    expect(list).toHaveLength(1);
  });

  it("cancel → POST .../cancel", async () => {
    await tasks.cancel("t1");
    expect(mock.lastRequest?.path).toBe("/api/v1/tasks/t1/cancel");
  });

  it("retry → POST .../retry", async () => {
    await tasks.retry("t1");
    expect(mock.lastRequest?.path).toBe("/api/v1/tasks/t1/retry");
  });
});

// ─────────────────────── Pipeline ───────────────────────

describe("PipelineResource", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let pipeline: PipelineResource;

  beforeEach(() => {
    mock = createMockTransport({
      "GET /api/v1/pipeline/status": { body: { active: true } },
      "POST /api/v1/pipeline/cancel": { body: {} },
      "GET /api/v1/pipeline/queue-metrics": { body: { pending: 5 } },
      "GET /api/v1/pipeline/costs/pricing": { body: [{ model: "gpt-4" }] },
      "POST /api/v1/pipeline/costs/estimate": { body: { cost: 0.05 } },
    });
    pipeline = new PipelineResource(mock as unknown as HttpTransport);
  });

  it("status → GET .../pipeline/status", async () => {
    const s = await pipeline.status();
    expect(s.active).toBe(true);
  });

  it("cancel → POST .../pipeline/cancel", async () => {
    await pipeline.cancel();
    expect(mock.lastRequest?.method).toBe("POST");
  });

  it("queueMetrics → GET .../pipeline/queue", async () => {
    const q = await pipeline.queueMetrics();
    expect(q.pending).toBe(5);
  });

  it("pricing → GET .../pipeline/pricing", async () => {
    const p = await pipeline.pricing();
    expect(p).toHaveLength(1);
  });

  it("estimateCost → POST .../estimate-cost", async () => {
    const est = await pipeline.estimateCost({ document_count: 10 });
    expect(est.cost).toBe(0.05);
  });
});

// ─────────────────────── Costs ───────────────────────

describe("CostsResource", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let costs: CostsResource;

  beforeEach(() => {
    mock = createMockTransport({
      "GET /api/v1/costs/summary": {
        body: {
          total_input_tokens: 10000,
          total_output_tokens: 5000,
          total_cost_usd: 1.5,
          formatted_cost: "$1.50",
          operations: [],
        },
      },
      "GET /api/v1/costs/history": { body: { data_points: [] } },
      "GET /api/v1/costs/budget": {
        body: {
          monthly_budget_usd: 100,
          spent_usd: 1.5,
          remaining_usd: 98.5,
          alert_threshold: 80,
          is_over_budget: false,
        },
      },
      "PATCH /api/v1/costs/budget": {
        body: {
          monthly_budget_usd: 200,
          spent_usd: 1.5,
          remaining_usd: 198.5,
          alert_threshold: 80,
          is_over_budget: false,
        },
      },
      "GET /api/v1/costs/pricing": { body: { models: [] } },
      "POST /api/v1/costs/estimate": {
        body: {
          model: "gpt-4",
          input_tokens: 1000,
          output_tokens: 500,
          estimated_cost_usd: 0.5,
          formatted_cost: "$0.50",
        },
      },
      "GET /api/v1/costs/workspace": {
        body: {
          workspace_id: "ws1",
          total_cost: 10.0,
          document_count: 5,
          total_tokens: 50000,
          average_cost_per_document: 2.0,
          by_operation: [],
        },
      },
    });
    costs = new CostsResource(mock as unknown as HttpTransport);
  });

  it("summary → GET .../costs/summary", async () => {
    const s = await costs.summary();
    expect(s.total_cost_usd).toBe(1.5);
    expect(s.formatted_cost).toBe("$1.50");
  });

  it("history → GET .../costs/history", async () => {
    await costs.history();
    expect(mock.lastRequest?.path).toBe("/api/v1/costs/history");
  });

  it("history with query → GET .../costs/history?start_date=...", async () => {
    await costs.history({ start_date: "2026-01-01", granularity: "day" });
    expect(mock.lastRequest?.path).toContain("start_date=2026-01-01");
    expect(mock.lastRequest?.path).toContain("granularity=day");
  });

  it("budget → GET .../costs/budget", async () => {
    const b = await costs.budget();
    expect(b.monthly_budget_usd).toBe(100);
    expect(b.is_over_budget).toBe(false);
  });

  it("updateBudget → PATCH .../costs/budget", async () => {
    await costs.updateBudget({ monthly_budget_usd: 200 });
    expect(mock.lastRequest?.method).toBe("PATCH");
  });

  it("pricing → GET .../costs/pricing", async () => {
    const p = await costs.pricing();
    expect(p.models).toBeDefined();
  });

  it("estimate → POST .../costs/estimate", async () => {
    const e = await costs.estimate({
      model: "gpt-4",
      input_tokens: 1000,
      output_tokens: 500,
    });
    expect(e.estimated_cost_usd).toBe(0.5);
  });

  it("workspaceSummary → GET .../costs/workspace", async () => {
    const ws = await costs.workspaceSummary();
    expect(ws.workspace_id).toBe("ws1");
    expect(ws.total_cost).toBe(10.0);
  });
});

// ─────────────────────── Lineage ───────────────────────

describe("LineageResource", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let lineage: LineageResource;

  beforeEach(() => {
    mock = createMockTransport({
      "GET /api/v1/lineage/entities/ENTITY_1": {
        body: {
          entity_name: "ENTITY_1",
          entity_type: "PERSON",
          source_documents: [],
          source_count: 0,
          description_versions: [],
        },
      },
      "GET /api/v1/lineage/documents/d1": {
        body: {
          document_id: "d1",
          chunk_count: 3,
          entities: [],
          relationships: [],
          extraction_stats: {
            total_entities: 0,
            unique_entities: 0,
            total_relationships: 0,
            unique_relationships: 0,
          },
        },
      },
    });
    lineage = new LineageResource(mock as unknown as HttpTransport);
  });

  it("entity → GET .../lineage/entities/:name", async () => {
    const res = await lineage.entity("ENTITY_1");
    expect(mock.lastRequest?.path).toBe("/api/v1/lineage/entities/ENTITY_1");
    expect(res.entity_name).toBe("ENTITY_1");
  });

  it("document → GET .../lineage/documents/:id", async () => {
    const res = await lineage.document("d1");
    expect(mock.lastRequest?.path).toBe("/api/v1/lineage/documents/d1");
    expect(res.document_id).toBe("d1");
    expect(res.chunk_count).toBe(3);
  });
});

// ─────────────────────── Chunks ───────────────────────

describe("ChunksResource", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let chunks: ChunksResource;

  beforeEach(() => {
    mock = createMockTransport({
      "GET /api/v1/chunks/c1": {
        body: {
          chunk_id: "c1",
          document_id: "d1",
          content: "Alice knows Bob",
          index: 0,
          char_range: { start: 0, end: 15 },
          token_count: 3,
          entities: [],
          relationships: [],
        },
      },
    });
    chunks = new ChunksResource(mock as unknown as HttpTransport);
  });

  it("get → GET /api/v1/chunks/:id", async () => {
    const c = await chunks.get("c1");
    expect(c.content).toBe("Alice knows Bob");
    expect(c.chunk_id).toBe("c1");
    expect(c.char_range.start).toBe(0);
  });
});

// ─────────────────────── Provenance ───────────────────────

describe("ProvenanceResource", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let prov: ProvenanceResource;

  beforeEach(() => {
    mock = createMockTransport({
      "GET /api/v1/entities/e1/provenance": {
        body: {
          entity_id: "e1",
          entity_name: "ENTITY_1",
          entity_type: "PERSON",
          sources: [],
          total_extraction_count: 3,
          related_entities: [],
        },
      },
    });
    prov = new ProvenanceResource(mock as unknown as HttpTransport);
  });

  it("get → GET /api/v1/entities/:id/provenance", async () => {
    const p = await prov.get("e1");
    expect(p.entity_id).toBe("e1");
    expect(p.entity_name).toBe("ENTITY_1");
    expect(p.total_extraction_count).toBe(3);
  });
});

// ─────────────────────── Settings ───────────────────────

describe("SettingsResource", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let settings: SettingsResource;

  beforeEach(() => {
    mock = createMockTransport({
      "GET /api/v1/settings/provider/status": { body: { providers: [] } },
      "GET /api/v1/settings/providers": { body: { providers: ["openai"] } },
    });
    settings = new SettingsResource(mock as unknown as HttpTransport);
  });

  it("providerStatus → GET .../provider/status", async () => {
    await settings.providerStatus();
    expect(mock.lastRequest?.path).toBe("/api/v1/settings/provider/status");
  });

  it("listProviders → GET .../providers", async () => {
    await settings.listProviders();
    expect(mock.lastRequest?.path).toBe("/api/v1/settings/providers");
  });
});

// ─────────────────────── Models ───────────────────────

describe("ModelsResource", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let models: ModelsResource;

  beforeEach(() => {
    mock = createMockTransport({
      "GET /api/v1/models": { body: [{ id: "m1" }] },
      "GET /api/v1/models/llm": { body: [{ id: "m1" }] },
      "GET /api/v1/models/embedding": { body: [{ id: "m2" }] },
      "GET /api/v1/models/health": { body: { healthy: true } },
      "GET /api/v1/models/openai": { body: [{ id: "m1" }] },
      "GET /api/v1/models/openai/gpt-4": { body: { id: "gpt-4" } },
    });
    models = new ModelsResource(mock as unknown as HttpTransport);
  });

  it("list → GET /api/v1/models", async () => {
    await models.list();
    expect(mock.lastRequest?.path).toBe("/api/v1/models");
  });

  it("listLlm → GET .../models/llm", async () => {
    await models.listLlm();
    expect(mock.lastRequest?.path).toBe("/api/v1/models/llm");
  });

  it("listEmbedding → GET .../models/embedding", async () => {
    await models.listEmbedding();
    expect(mock.lastRequest?.path).toBe("/api/v1/models/embedding");
  });

  it("health → GET .../models/health", async () => {
    await models.health();
    expect(mock.lastRequest?.path).toBe("/api/v1/models/health");
  });

  it("getProvider → GET .../providers/:name", async () => {
    await models.getProvider("openai");
    expect(mock.lastRequest?.path).toBe("/api/v1/models/openai");
  });

  it("getModel → GET .../models/:p/:m", async () => {
    await models.getModel("openai", "gpt-4");
    expect(mock.lastRequest?.path).toBe("/api/v1/models/openai/gpt-4");
  });
});

// ─────────────────────── Ollama ───────────────────────

describe("OllamaResource", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let ollama: OllamaResource;

  beforeEach(() => {
    mock = createMockTransport({
      "GET /api/version": { body: { version: "0.1.0" } },
      "GET /api/tags": { body: [{ name: "llama2" }] },
      "GET /api/ps": { body: [{ name: "llama2" }] },
      "POST /api/generate": { body: { response: "Hello" } },
      "POST /api/chat": { body: { message: { content: "Hi" } } },
    });
    ollama = new OllamaResource(mock as unknown as HttpTransport);
  });

  it("version → GET /api/version", async () => {
    const v = await ollama.version();
    expect(v.version).toBe("0.1.0");
  });

  it("tags → GET /api/tags", async () => {
    const tags = await ollama.tags();
    expect(tags).toHaveLength(1);
  });

  it("ps → GET /api/ps", async () => {
    const ps = await ollama.ps();
    expect(ps).toHaveLength(1);
  });

  it("generate → POST /api/generate", async () => {
    const res = await ollama.generate({ model: "llama2", prompt: "hi" });
    expect(res.response).toBe("Hello");
  });

  it("chat → POST /api/chat", async () => {
    const res = await ollama.chat({
      model: "llama2",
      messages: [{ role: "user", content: "hi" }],
    });
    expect(res.message.content).toBe("Hi");
  });
});

// ─────────────────────── Client with mock transport ───────────────────────

describe("EdgeQuake with _transport", () => {
  it("uses injected transport instead of creating one", async () => {
    const { EdgeQuake } = await import("../../src/client.js");
    const mock = createMockTransport({
      "GET /health": { body: { status: "healthy" } },
    });
    const client = new EdgeQuake({
      _transport: mock as unknown as HttpTransport,
    });
    const health = await client.health();
    expect(health.status).toBe("healthy");
    expect(mock.requests).toHaveLength(1);
  });
});

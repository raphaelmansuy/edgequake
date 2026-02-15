<?php

declare(strict_types=1);

namespace EdgeQuake;

// WHY: Each service maps 1:1 to an API resource for discoverability.

class HealthService
{
    public function __construct(private readonly HttpHelper $http) {}
    public function check(): array { return $this->http->get('/health'); }
    public function ready(): array { return $this->http->get('/ready'); }
    public function live(): array { return $this->http->get('/live'); }
    public function metrics(): string { return $this->http->getRaw('/metrics'); }
}

class DocumentService
{
    public function __construct(private readonly HttpHelper $http) {}

    public function list(int $page = 1, int $pageSize = 20): array
    {
        return $this->http->get("/api/v1/documents?page={$page}&page_size={$pageSize}");
    }

    public function get(string $id): array
    {
        return $this->http->get("/api/v1/documents/{$id}");
    }

    public function uploadText(string $title, string $content, string $fileType = 'txt'): array
    {
        return $this->http->post('/api/v1/documents', [
            'title' => $title, 'content' => $content, 'file_type' => $fileType,
        ]);
    }

    public function delete(string $id): array
    {
        return $this->http->delete("/api/v1/documents/{$id}");
    }

    public function deleteAll(): array
    {
        return $this->http->delete('/api/v1/documents');
    }

    public function reprocess(): array
    {
        return $this->http->post('/api/v1/documents/reprocess');
    }

    public function recoverStuck(): array
    {
        return $this->http->post('/api/v1/documents/recover-stuck');
    }

    public function retryChunks(string $id): array
    {
        return $this->http->post("/api/v1/documents/{$id}/retry-chunks");
    }

    public function failedChunks(string $id): array
    {
        return $this->http->get("/api/v1/documents/{$id}/failed-chunks");
    }

    // OODA-39: Additional document methods.

    /** Get document chunks with pagination. */
    public function chunks(string $id, int $page = 1, int $pageSize = 20): array
    {
        return $this->http->get("/api/v1/documents/{$id}/chunks?page={$page}&page_size={$pageSize}");
    }

    /** Get document processing status. */
    public function status(string $id): array
    {
        return $this->http->get("/api/v1/documents/{$id}/status");
    }

    /** Get document metadata. */
    public function getMetadata(string $id): array
    {
        return $this->http->get("/api/v1/documents/{$id}/metadata");
    }

    /** Update document metadata. */
    public function setMetadata(string $id, array $metadata): array
    {
        return $this->http->patch("/api/v1/documents/{$id}/metadata", ['metadata' => $metadata]);
    }

    /** Upload PDF document. */
    public function uploadPdf(string $filePath, ?string $title = null): array
    {
        return $this->http->upload('/api/v1/documents/pdf/upload', $filePath, 'file', $title ? ['title' => $title] : []);
    }

    /** Get PDF extraction status. */
    public function pdfStatus(string $id): array
    {
        return $this->http->get("/api/v1/documents/pdf/{$id}/status");
    }

    /** Download extracted PDF markdown. */
    public function pdfDownload(string $id): string
    {
        return $this->http->getRaw("/api/v1/documents/pdf/{$id}/download");
    }
}

class EntityService
{
    public function __construct(private readonly HttpHelper $http) {}

    public function list(int $page = 1, int $pageSize = 20): array
    {
        return $this->http->get("/api/v1/graph/entities?page={$page}&page_size={$pageSize}");
    }

    public function get(string $name): array
    {
        return $this->http->get("/api/v1/graph/entities/{$name}");
    }

    public function create(string $entityName, string $entityType, string $description, string $sourceId): array
    {
        return $this->http->post('/api/v1/graph/entities', [
            'entity_name' => $entityName, 'entity_type' => $entityType,
            'description' => $description, 'source_id' => $sourceId,
        ]);
    }

    public function delete(string $name): array
    {
        return $this->http->delete("/api/v1/graph/entities/{$name}?confirm=true");
    }

    public function update(string $name, array $data): array
    {
        $encoded = rawurlencode($name);
        return $this->http->put("/api/v1/graph/entities/{$encoded}", $data);
    }

    public function merge(string $sourceName, string $targetName): array
    {
        return $this->http->post('/api/v1/graph/entities/merge', [
            'source_name' => $sourceName,
            'target_name' => $targetName,
        ]);
    }

    public function neighborhood(string $name, int $depth = 1): array
    {
        $encoded = rawurlencode($name);
        return $this->http->get("/api/v1/graph/entities/{$encoded}/neighborhood?depth={$depth}");
    }

    // OODA-39: Get available entity types.

    /** Get list of entity types. */
    public function types(): array
    {
        return $this->http->get('/api/v1/graph/entities/types');
    }
}

class RelationshipService
{
    public function __construct(private readonly HttpHelper $http) {}

    public function list(int $page = 1, int $pageSize = 20): array
    {
        return $this->http->get("/api/v1/graph/relationships?page={$page}&page_size={$pageSize}");
    }

    public function get(string $id): array
    {
        $encoded = rawurlencode($id);
        return $this->http->get("/api/v1/graph/relationships/{$encoded}");
    }

    public function update(string $id, array $data): array
    {
        $encoded = rawurlencode($id);
        return $this->http->put("/api/v1/graph/relationships/{$encoded}", $data);
    }

    public function delete(string $id): array
    {
        $encoded = rawurlencode($id);
        return $this->http->delete("/api/v1/graph/relationships/{$encoded}");
    }

    // OODA-39: Get relationship types.

    /** Get list of relationship types. */
    public function types(): array
    {
        return $this->http->get('/api/v1/graph/relationships/types');
    }
}

class GraphService
{
    public function __construct(private readonly HttpHelper $http) {}

    public function get(): array
    {
        return $this->http->get('/api/v1/graph');
    }

    public function search(string $query): array
    {
        $encoded = urlencode($query);
        return $this->http->get("/api/v1/graph/nodes/search?q={$encoded}");
    }

    public function getNode(string $id): array
    {
        $encoded = rawurlencode($id);
        return $this->http->get("/api/v1/graph/nodes/{$encoded}");
    }

    public function searchLabels(string $query): array
    {
        $encoded = urlencode($query);
        return $this->http->get("/api/v1/graph/labels/search?q={$encoded}");
    }

    public function popularLabels(int $limit = 20): array
    {
        return $this->http->get("/api/v1/graph/labels/popular?limit={$limit}");
    }

    public function degreesBatch(array $nodeIds): array
    {
        return $this->http->post('/api/v1/graph/degrees/batch', ['node_ids' => $nodeIds]);
    }

    // OODA-39: Additional graph methods.

    /** Get graph statistics. */
    public function stats(): array
    {
        return $this->http->get('/api/v1/graph/stats');
    }

    /** Clear all graph data. */
    public function clear(): array
    {
        return $this->http->post('/api/v1/graph/clear');
    }
}

class QueryService
{
    public function __construct(private readonly HttpHelper $http) {}

    public function execute(string $query, string $mode = 'hybrid'): array
    {
        return $this->http->post('/api/v1/query', ['query' => $query, 'mode' => $mode]);
    }

    // OODA-39: Streaming query.

    /** Execute streaming query. Returns generator of chunks. */
    public function stream(string $query, string $mode = 'hybrid'): \Generator
    {
        return $this->http->streamPost('/api/v1/query/stream', ['query' => $query, 'mode' => $mode]);
    }
}

class ChatService
{
    public function __construct(private readonly HttpHelper $http) {}

    public function completions(string $message, string $mode = 'hybrid', bool $stream = false): array
    {
        return $this->http->post('/api/v1/chat/completions', [
            'message' => $message, 'mode' => $mode, 'stream' => $stream,
        ]);
    }

    // OODA-39: Streaming chat and conversation support.

    /** Streaming chat completions. Returns generator of chunks. */
    public function stream(string $message, string $mode = 'hybrid'): \Generator
    {
        return $this->http->streamPost('/api/v1/chat/completions/stream', [
            'message' => $message, 'mode' => $mode,
        ]);
    }

    /** Chat completions with conversation context. */
    public function completionsWithConversation(string $conversationId, string $message, string $mode = 'hybrid'): array
    {
        return $this->http->post('/api/v1/chat/completions', [
            'conversation_id' => $conversationId, 'message' => $message, 'mode' => $mode,
        ]);
    }
}

class TenantService
{
    public function __construct(private readonly HttpHelper $http) {}
    public function list(): array { return $this->http->get('/api/v1/tenants'); }
    public function get(string $id): array { return $this->http->get("/api/v1/tenants/{$id}"); }
    public function create(string $name): array { return $this->http->post('/api/v1/tenants', ['name' => $name]); }
    public function update(string $id, array $data): array { return $this->http->put("/api/v1/tenants/{$id}", $data); }
    public function delete(string $id): array { return $this->http->delete("/api/v1/tenants/{$id}"); }
}

class UserService
{
    public function __construct(private readonly HttpHelper $http) {}
    public function list(): array { return $this->http->get('/api/v1/users'); }
    public function get(string $id): array { return $this->http->get("/api/v1/users/{$id}"); }
    public function create(string $username, string $email): array
    {
        return $this->http->post('/api/v1/users', ['username' => $username, 'email' => $email]);
    }
    // OODA-39: Update user.
    public function update(string $id, array $data): array { return $this->http->put("/api/v1/users/{$id}", $data); }
    public function delete(string $id): array { return $this->http->delete("/api/v1/users/{$id}"); }
}

class ApiKeyService
{
    public function __construct(private readonly HttpHelper $http) {}
    public function list(): array { return $this->http->get('/api/v1/api-keys'); }
    public function create(string $name): array { return $this->http->post('/api/v1/api-keys', ['name' => $name]); }
    public function delete(string $id): array { return $this->http->delete("/api/v1/api-keys/{$id}"); }
    public function revoke(string $id): array { return $this->http->post("/api/v1/api-keys/{$id}/revoke"); }
}

class TaskService
{
    public function __construct(private readonly HttpHelper $http) {}
    public function list(): array { return $this->http->get('/api/v1/tasks'); }
    public function get(string $id): array { return $this->http->get("/api/v1/tasks/{$id}"); }
    public function cancel(string $id): array { return $this->http->post("/api/v1/tasks/{$id}/cancel"); }
    public function retry(string $id): array { return $this->http->post("/api/v1/tasks/{$id}/retry"); }
}

class PipelineService
{
    public function __construct(private readonly HttpHelper $http) {}

    public function status(): array
    {
        return $this->http->get('/api/v1/pipeline/status');
    }

    public function queueMetrics(): array
    {
        return $this->http->get('/api/v1/pipeline/queue-metrics');
    }

    public function cancel(): array
    {
        return $this->http->post('/api/v1/pipeline/cancel');
    }
}

class ModelService
{
    public function __construct(private readonly HttpHelper $http) {}

    public function catalog(): array
    {
        return $this->http->get('/api/v1/models');
    }

    public function health(): array
    {
        $raw = $this->http->getRaw('/api/v1/models/health');
        return json_decode($raw, true) ?? [];
    }

    public function providerStatus(): array
    {
        return $this->http->get('/api/v1/settings/provider/status');
    }

    public function listLlm(): array
    {
        return $this->http->get('/api/v1/models/llm');
    }

    public function listEmbedding(): array
    {
        return $this->http->get('/api/v1/models/embedding');
    }

    public function getProvider(string $provider): array
    {
        $encoded = rawurlencode($provider);
        return $this->http->get("/api/v1/models/{$encoded}");
    }

    public function getModel(string $provider, string $model): array
    {
        $pEncoded = rawurlencode($provider);
        $mEncoded = rawurlencode($model);
        return $this->http->get("/api/v1/models/{$pEncoded}/{$mEncoded}");
    }

    public function listProviders(): array
    {
        return $this->http->get('/api/v1/settings/providers');
    }
}

class CostService
{
    public function __construct(private readonly HttpHelper $http) {}

    public function summary(): array
    {
        return $this->http->get('/api/v1/costs/summary');
    }

    public function history(): array
    {
        return $this->http->get('/api/v1/costs/history');
    }

    public function pricing(): array
    {
        return $this->http->get('/api/v1/pipeline/costs/pricing');
    }

    public function estimate(array $params): array
    {
        return $this->http->post('/api/v1/pipeline/costs/estimate', $params);
    }

    public function updateBudget(array $budget): array
    {
        return $this->http->patch('/api/v1/costs/budget', $budget);
    }
}

class ConversationService
{
    public function __construct(private readonly HttpHelper $http) {}

    public function list(): array
    {
        return $this->http->get('/api/v1/conversations');
    }

    public function get(string $id): array
    {
        return $this->http->get("/api/v1/conversations/{$id}");
    }

    public function create(string $title, ?string $mode = null, ?string $folderId = null): array
    {
        $body = ['title' => $title];
        if ($mode !== null) $body['mode'] = $mode;
        if ($folderId !== null) $body['folder_id'] = $folderId;
        return $this->http->post('/api/v1/conversations', $body);
    }

    public function update(string $id, array $data): array
    {
        return $this->http->patch("/api/v1/conversations/{$id}", $data);
    }

    public function delete(string $id): array
    {
        return $this->http->delete("/api/v1/conversations/{$id}");
    }

    public function import(array $data): array
    {
        return $this->http->post('/api/v1/conversations/import', $data);
    }

    public function share(string $id): array
    {
        return $this->http->post("/api/v1/conversations/{$id}/share");
    }

    public function unshare(string $id): array
    {
        return $this->http->delete("/api/v1/conversations/{$id}/share");
    }

    public function listMessages(string $id): array
    {
        return $this->http->get("/api/v1/conversations/{$id}/messages");
    }

    public function createMessage(string $id, string $content, string $role = 'user'): array
    {
        return $this->http->post("/api/v1/conversations/{$id}/messages", [
            'content' => $content,
            'role' => $role,
        ]);
    }

    // OODA-39: Update and delete messages.

    /** Update a message in conversation. */
    public function updateMessage(string $id, string $messageId, string $content): array
    {
        return $this->http->patch("/api/v1/conversations/{$id}/messages/{$messageId}", [
            'content' => $content,
        ]);
    }

    /** Delete a message from conversation. */
    public function deleteMessage(string $id, string $messageId): array
    {
        return $this->http->delete("/api/v1/conversations/{$id}/messages/{$messageId}");
    }

    public function bulkArchive(array $ids): array
    {
        return $this->http->post('/api/v1/conversations/bulk/archive', ['ids' => $ids]);
    }

    public function bulkMove(array $ids, string $folderId): array
    {
        return $this->http->post('/api/v1/conversations/bulk/move', [
            'ids' => $ids,
            'folder_id' => $folderId,
        ]);
    }
}

class FolderService
{
    public function __construct(private readonly HttpHelper $http) {}

    public function list(): array
    {
        return $this->http->get('/api/v1/folders');
    }

    public function get(string $id): array
    {
        return $this->http->get("/api/v1/folders/{$id}");
    }

    public function create(string $name): array
    {
        return $this->http->post('/api/v1/folders', ['name' => $name]);
    }

    public function update(string $id, array $data): array
    {
        return $this->http->patch("/api/v1/folders/{$id}", $data);
    }

    public function delete(string $id): array
    {
        return $this->http->delete("/api/v1/folders/{$id}");
    }
}

// WHY: Auth service — login, logout, refresh, me.
// OODA-33: PHP SDK auth service.
class AuthService
{
    public function __construct(private readonly HttpHelper $http) {}

    public function login(string $username, string $password): array
    {
        return $this->http->post('/api/v1/auth/login', [
            'username' => $username,
            'password' => $password,
        ]);
    }

    public function logout(): array
    {
        return $this->http->post('/api/v1/auth/logout');
    }

    public function refresh(string $refreshToken): array
    {
        return $this->http->post('/api/v1/auth/refresh', [
            'refresh_token' => $refreshToken,
        ]);
    }

    public function me(): array
    {
        return $this->http->get('/api/v1/auth/me');
    }
}

// WHY: Workspace service — CRUD + rebuild + metrics.
// OODA-33: PHP SDK workspace service.
class WorkspaceService
{
    public function __construct(private readonly HttpHelper $http) {}

    public function list(): array
    {
        return $this->http->get('/api/v1/workspaces');
    }

    public function get(string $id): array
    {
        return $this->http->get("/api/v1/workspaces/{$id}");
    }

    public function create(string $name, ?string $tenantId = null): array
    {
        $body = ['name' => $name];
        if ($tenantId !== null) $body['tenant_id'] = $tenantId;
        return $this->http->post('/api/v1/workspaces', $body);
    }

    public function update(string $id, array $data): array
    {
        return $this->http->put("/api/v1/workspaces/{$id}", $data);
    }

    public function delete(string $id): array
    {
        return $this->http->delete("/api/v1/workspaces/{$id}");
    }

    public function metricsHistory(string $id): array
    {
        return $this->http->get("/api/v1/workspaces/{$id}/metrics-history");
    }

    public function rebuildEmbeddings(string $id): array
    {
        return $this->http->post("/api/v1/workspaces/{$id}/rebuild-embeddings");
    }

    public function rebuildKnowledgeGraph(string $id): array
    {
        return $this->http->post("/api/v1/workspaces/{$id}/rebuild-knowledge-graph");
    }

    public function reprocessDocuments(string $id): array
    {
        return $this->http->post("/api/v1/workspaces/{$id}/reprocess-documents");
    }

    // OODA-39: Workspace stats.

    /** Get workspace statistics. */
    public function stats(string $id): array
    {
        return $this->http->get("/api/v1/workspaces/{$id}/stats");
    }
}

// WHY: Shared conversations service — public read access.
// OODA-33: PHP SDK shared service.
class SharedService
{
    public function __construct(private readonly HttpHelper $http) {}

    public function get(string $shareId): array
    {
        return $this->http->get("/api/v1/shared/{$shareId}");
    }
}

// WHY: Lineage & provenance service — maps 7 lineage API endpoints.
// OODA-27: PHP SDK lineage service.
class LineageService
{
    public function __construct(private readonly HttpHelper $http) {}

    /** Get entity lineage showing all source documents. */
    public function entityLineage(string $name): array
    {
        $encoded = rawurlencode($name);
        return $this->http->get("/api/v1/lineage/entities/{$encoded}");
    }

    /** Get document graph lineage with entities and relationships. */
    public function documentLineage(string $id): array
    {
        $encoded = rawurlencode($id);
        return $this->http->get("/api/v1/lineage/documents/{$encoded}");
    }

    /** Get full document lineage including metadata. */
    public function documentFullLineage(string $id): array
    {
        $encoded = rawurlencode($id);
        return $this->http->get("/api/v1/documents/{$encoded}/lineage");
    }

    /** Export document lineage as JSON or CSV. Returns raw string. */
    public function exportLineage(string $id, string $format = 'json'): string
    {
        $encoded = rawurlencode($id);
        $fmtEncoded = rawurlencode($format);
        return $this->http->getRaw("/api/v1/documents/{$encoded}/lineage/export?format={$fmtEncoded}");
    }

    /** Get detailed chunk information with extracted entities. */
    public function chunkDetail(string $id): array
    {
        $encoded = rawurlencode($id);
        return $this->http->get("/api/v1/chunks/{$encoded}");
    }

    /** Get chunk lineage with parent document references. */
    public function chunkLineage(string $id): array
    {
        $encoded = rawurlencode($id);
        return $this->http->get("/api/v1/chunks/{$encoded}/lineage");
    }

    /** Get entity provenance with source documents and related entities. */
    public function entityProvenance(string $id): array
    {
        $encoded = rawurlencode($id);
        return $this->http->get("/api/v1/entities/{$encoded}/provenance");
    }
}

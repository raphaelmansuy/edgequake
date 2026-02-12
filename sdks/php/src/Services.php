<?php

declare(strict_types=1);

namespace EdgeQuake;

// WHY: Each service maps 1:1 to an API resource for discoverability.

class HealthService
{
    public function __construct(private readonly HttpHelper $http) {}
    public function check(): array { return $this->http->get('/health'); }
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
}

class RelationshipService
{
    public function __construct(private readonly HttpHelper $http) {}

    public function list(int $page = 1, int $pageSize = 20): array
    {
        return $this->http->get("/api/v1/graph/relationships?page={$page}&page_size={$pageSize}");
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
}

class QueryService
{
    public function __construct(private readonly HttpHelper $http) {}

    public function execute(string $query, string $mode = 'hybrid'): array
    {
        return $this->http->post('/api/v1/query', ['query' => $query, 'mode' => $mode]);
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
}

class TenantService
{
    public function __construct(private readonly HttpHelper $http) {}
    public function list(): array { return $this->http->get('/api/v1/tenants'); }
}

class UserService
{
    public function __construct(private readonly HttpHelper $http) {}
    public function list(): array { return $this->http->get('/api/v1/users'); }
}

class ApiKeyService
{
    public function __construct(private readonly HttpHelper $http) {}
    public function list(): array { return $this->http->get('/api/v1/api-keys'); }
}

class TaskService
{
    public function __construct(private readonly HttpHelper $http) {}
    public function list(): array { return $this->http->get('/api/v1/tasks'); }
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
}

class CostService
{
    public function __construct(private readonly HttpHelper $http) {}

    public function summary(): array
    {
        return $this->http->get('/api/v1/costs/summary');
    }
}

class ConversationService
{
    public function __construct(private readonly HttpHelper $http) {}

    public function list(): array
    {
        return $this->http->get('/api/v1/conversations');
    }

    public function create(string $title, ?string $mode = null, ?string $folderId = null): array
    {
        $body = ['title' => $title];
        if ($mode !== null) $body['mode'] = $mode;
        if ($folderId !== null) $body['folder_id'] = $folderId;
        return $this->http->post('/api/v1/conversations', $body);
    }
}

class FolderService
{
    public function __construct(private readonly HttpHelper $http) {}

    public function list(): array
    {
        return $this->http->get('/api/v1/folders');
    }

    public function create(string $name): array
    {
        return $this->http->post('/api/v1/folders', ['name' => $name]);
    }
}

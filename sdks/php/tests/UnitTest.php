<?php

declare(strict_types=1);

namespace EdgeQuake\Tests;

use PHPUnit\Framework\TestCase;
use EdgeQuake\Config;
use EdgeQuake\Client;
use EdgeQuake\ApiError;
use EdgeQuake\HealthService;
use EdgeQuake\DocumentService;
use EdgeQuake\EntityService;
use EdgeQuake\RelationshipService;
use EdgeQuake\GraphService;
use EdgeQuake\QueryService;
use EdgeQuake\ChatService;
use EdgeQuake\TenantService;
use EdgeQuake\UserService;
use EdgeQuake\ApiKeyService;
use EdgeQuake\TaskService;
use EdgeQuake\PipelineService;
use EdgeQuake\ModelService;
use EdgeQuake\CostService;
use EdgeQuake\ConversationService;
use EdgeQuake\FolderService;
use EdgeQuake\LineageService;
use EdgeQuake\AuthService;
use EdgeQuake\WorkspaceService;
use EdgeQuake\SharedService;

/**
 * Unit tests for the EdgeQuake PHP SDK.
 * WHY: Verify all components without making real HTTP calls.
 */
class UnitTest extends TestCase
{
    // ── Config Tests ───────────────────────────────────────────────

    public function testConfigDefaults(): void
    {
        $config = new Config();
        $this->assertSame('http://localhost:8080', $config->baseUrl);
        $this->assertNull($config->apiKey);
        $this->assertNull($config->tenantId);
        $this->assertNull($config->userId);
        $this->assertNull($config->workspaceId);
        $this->assertSame(60, $config->timeout);
    }

    public function testConfigCustomValues(): void
    {
        $config = new Config(
            baseUrl: 'https://api.example.com',
            apiKey: 'sk-test',
            tenantId: 't-1',
            userId: 'u-1',
            workspaceId: 'ws-1',
            timeout: 120,
        );
        $this->assertSame('https://api.example.com', $config->baseUrl);
        $this->assertSame('sk-test', $config->apiKey);
        $this->assertSame('t-1', $config->tenantId);
        $this->assertSame('u-1', $config->userId);
        $this->assertSame('ws-1', $config->workspaceId);
        $this->assertSame(120, $config->timeout);
    }

    // ── ApiError Tests ─────────────────────────────────────────────

    public function testApiErrorMessage(): void
    {
        $err = new ApiError('something broke', statusCode: 500, responseBody: '{"error":"fail"}');
        $this->assertSame('something broke', $err->getMessage());
        $this->assertSame(500, $err->statusCode);
        $this->assertSame('{"error":"fail"}', $err->responseBody);
    }

    public function testApiErrorIsRuntimeException(): void
    {
        $err = new ApiError('test');
        $this->assertInstanceOf(\RuntimeException::class, $err);
    }

    public function testApiErrorNullDefaults(): void
    {
        $err = new ApiError('test');
        $this->assertNull($err->statusCode);
        $this->assertNull($err->responseBody);
    }

    // ── Client Tests ───────────────────────────────────────────────

    public function testClientInitializesAllServices(): void
    {
        $client = new Client();
        $this->assertInstanceOf(HealthService::class, $client->health);
        $this->assertInstanceOf(AuthService::class, $client->auth);
        $this->assertInstanceOf(DocumentService::class, $client->documents);
        $this->assertInstanceOf(EntityService::class, $client->entities);
        $this->assertInstanceOf(RelationshipService::class, $client->relationships);
        $this->assertInstanceOf(GraphService::class, $client->graph);
        $this->assertInstanceOf(QueryService::class, $client->query);
        $this->assertInstanceOf(ChatService::class, $client->chat);
        $this->assertInstanceOf(TenantService::class, $client->tenants);
        $this->assertInstanceOf(UserService::class, $client->users);
        $this->assertInstanceOf(ApiKeyService::class, $client->apiKeys);
        $this->assertInstanceOf(TaskService::class, $client->tasks);
        $this->assertInstanceOf(PipelineService::class, $client->pipeline);
        $this->assertInstanceOf(ModelService::class, $client->models);
        $this->assertInstanceOf(CostService::class, $client->costs);
        $this->assertInstanceOf(ConversationService::class, $client->conversations);
        $this->assertInstanceOf(FolderService::class, $client->folders);
        $this->assertInstanceOf(WorkspaceService::class, $client->workspaces);
        $this->assertInstanceOf(SharedService::class, $client->shared);
        $this->assertInstanceOf(LineageService::class, $client->lineage);
    }

    public function testClientWithCustomConfig(): void
    {
        $config = new Config(baseUrl: 'https://test.api');
        $client = new Client($config);
        $this->assertInstanceOf(HealthService::class, $client->health);
    }

    // ── Health Service ─────────────────────────────────────────────

    public function testHealthCheck(): void
    {
        $mock = new MockHttpHelper('{"status":"healthy","version":"0.1.0"}');
        $svc = new HealthService($mock);
        $result = $svc->check();
        $this->assertSame('healthy', $result['status']);
        $this->assertSame('0.1.0', $result['version']);
        $this->assertSame('GET', $mock->lastCall()['method']);
        $this->assertSame('/health', $mock->lastCall()['path']);
    }

    // ── Document Service ───────────────────────────────────────────

    public function testDocumentsList(): void
    {
        $mock = new MockHttpHelper('{"documents":[{"id":"d1"}]}');
        $svc = new DocumentService($mock);
        $result = $svc->list(1, 10);
        $this->assertCount(1, $result['documents']);
        $this->assertStringContainsString('page=1', $mock->lastCall()['path']);
        $this->assertStringContainsString('page_size=10', $mock->lastCall()['path']);
    }

    public function testDocumentsGet(): void
    {
        $mock = new MockHttpHelper('{"id":"d1","file_name":"test.pdf"}');
        $svc = new DocumentService($mock);
        $result = $svc->get('d1');
        $this->assertSame('d1', $result['id']);
        $this->assertStringContainsString('/api/v1/documents/d1', $mock->lastCall()['path']);
    }

    public function testDocumentsUploadText(): void
    {
        $mock = new MockHttpHelper('{"id":"d2","status":"processing"}');
        $svc = new DocumentService($mock);
        $result = $svc->uploadText('My Title', 'Hello World', 'txt');
        $this->assertSame('d2', $result['id']);
        $this->assertSame('POST', $mock->lastCall()['method']);
        $this->assertSame('My Title', $mock->lastCall()['body']['title']);
        $this->assertSame('Hello World', $mock->lastCall()['body']['content']);
    }

    public function testDocumentsDelete(): void
    {
        $mock = new MockHttpHelper('{"status":"deleted"}');
        $svc = new DocumentService($mock);
        $svc->delete('d1');
        $this->assertSame('DELETE', $mock->lastCall()['method']);
    }

    public function testDocumentsListError(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{"error":"Internal Server Error"}', 500);
        $svc = new DocumentService($mock);
        $this->expectException(ApiError::class);
        $svc->list();
    }

    public function testDocumentsGetError(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{"error":"Not Found"}', 404);
        $svc = new DocumentService($mock);
        $this->expectException(ApiError::class);
        $svc->get('missing');
    }

    // ── Entity Service ─────────────────────────────────────────────

    public function testEntitiesList(): void
    {
        $mock = new MockHttpHelper('{"items":[{"entity_name":"ALICE"}],"total":1}');
        $svc = new EntityService($mock);
        $result = $svc->list(1, 20);
        $this->assertCount(1, $result['items']);
        $this->assertSame('ALICE', $result['items'][0]['entity_name']);
    }

    public function testEntitiesGet(): void
    {
        $mock = new MockHttpHelper('{"entity_name":"ALICE","entity_type":"person"}');
        $svc = new EntityService($mock);
        $result = $svc->get('ALICE');
        $this->assertSame('person', $result['entity_type']);
    }

    public function testEntitiesCreate(): void
    {
        $mock = new MockHttpHelper('{"status":"success","entity":{"entity_name":"BOB"}}');
        $svc = new EntityService($mock);
        $result = $svc->create('BOB', 'person', 'A person', 'manual');
        $this->assertSame('success', $result['status']);
        $this->assertSame('POST', $mock->lastCall()['method']);
    }

    public function testEntitiesDelete(): void
    {
        $mock = new MockHttpHelper('{"status":"deleted"}');
        $svc = new EntityService($mock);
        $svc->delete('ALICE');
        $this->assertSame('DELETE', $mock->lastCall()['method']);
        $this->assertStringContainsString('confirm=true', $mock->lastCall()['path']);
    }

    public function testEntitiesListError(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{}', 500);
        $svc = new EntityService($mock);
        $this->expectException(ApiError::class);
        $svc->list();
    }

    // ── Relationship Service ───────────────────────────────────────

    public function testRelationshipsList(): void
    {
        $mock = new MockHttpHelper('{"items":[{"source":"A","target":"B"}],"total":1}');
        $svc = new RelationshipService($mock);
        $result = $svc->list();
        $this->assertCount(1, $result['items']);
    }

    public function testRelationshipsListError(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{}', 500);
        $svc = new RelationshipService($mock);
        $this->expectException(ApiError::class);
        $svc->list();
    }

    // ── Graph Service ──────────────────────────────────────────────

    public function testGraphGet(): void
    {
        $mock = new MockHttpHelper('{"nodes":[],"edges":[]}');
        $svc = new GraphService($mock);
        $result = $svc->get();
        $this->assertArrayHasKey('nodes', $result);
    }

    public function testGraphSearch(): void
    {
        $mock = new MockHttpHelper('{"nodes":[{"id":"n1"}]}');
        $svc = new GraphService($mock);
        $result = $svc->search('Alice');
        $this->assertCount(1, $result['nodes']);
        $this->assertStringContainsString('q=Alice', $mock->lastCall()['path']);
    }

    public function testGraphSearchUrlEncoding(): void
    {
        $mock = new MockHttpHelper('{"nodes":[]}');
        $svc = new GraphService($mock);
        $svc->search('hello world');
        $this->assertStringContainsString('q=hello+world', $mock->lastCall()['path']);
    }

    public function testGraphGetError(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{}', 500);
        $svc = new GraphService($mock);
        $this->expectException(ApiError::class);
        $svc->get();
    }

    // ── Query Service ──────────────────────────────────────────────

    public function testQueryExecute(): void
    {
        $mock = new MockHttpHelper('{"answer":"42","sources":[]}');
        $svc = new QueryService($mock);
        $result = $svc->execute('meaning of life', 'hybrid');
        $this->assertSame('42', $result['answer']);
        $this->assertSame('POST', $mock->lastCall()['method']);
        $this->assertSame('meaning of life', $mock->lastCall()['body']['query']);
    }

    public function testQueryExecuteError(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{}', 500);
        $svc = new QueryService($mock);
        $this->expectException(ApiError::class);
        $svc->execute('test');
    }

    // ── Chat Service ───────────────────────────────────────────────

    public function testChatCompletions(): void
    {
        $mock = new MockHttpHelper('{"choices":[{"message":{"content":"Hello!"}}]}');
        $svc = new ChatService($mock);
        $result = $svc->completions('Hi');
        $this->assertCount(1, $result['choices']);
        $this->assertSame('POST', $mock->lastCall()['method']);
    }

    public function testChatCompletionsError(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{}', 500);
        $svc = new ChatService($mock);
        $this->expectException(ApiError::class);
        $svc->completions('test');
    }

    // ── Tenant Service ─────────────────────────────────────────────

    public function testTenantsList(): void
    {
        $mock = new MockHttpHelper('{"items":[{"id":"t1","name":"Acme"}]}');
        $svc = new TenantService($mock);
        $result = $svc->list();
        $this->assertCount(1, $result['items']);
    }

    public function testTenantsListError(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{}', 500);
        $svc = new TenantService($mock);
        $this->expectException(ApiError::class);
        $svc->list();
    }

    // ── User Service ───────────────────────────────────────────────

    public function testUsersList(): void
    {
        $mock = new MockHttpHelper('[{"id":"u1","username":"admin"}]');
        $svc = new UserService($mock);
        $result = $svc->list();
        $this->assertCount(1, $result);
    }

    public function testUsersListError(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{}', 500);
        $svc = new UserService($mock);
        $this->expectException(ApiError::class);
        $svc->list();
    }

    // ── API Key Service ────────────────────────────────────────────

    public function testApiKeysList(): void
    {
        $mock = new MockHttpHelper('[{"id":"ak-1","name":"key1"}]');
        $svc = new ApiKeyService($mock);
        $result = $svc->list();
        $this->assertCount(1, $result);
    }

    public function testApiKeysListError(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{}', 500);
        $svc = new ApiKeyService($mock);
        $this->expectException(ApiError::class);
        $svc->list();
    }

    // ── Task Service ───────────────────────────────────────────────

    public function testTasksList(): void
    {
        $mock = new MockHttpHelper('{"tasks":[{"track_id":"trk-1"}]}');
        $svc = new TaskService($mock);
        $result = $svc->list();
        $this->assertCount(1, $result['tasks']);
    }

    public function testTasksListError(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{}', 500);
        $svc = new TaskService($mock);
        $this->expectException(ApiError::class);
        $svc->list();
    }

    // ── Pipeline Service ───────────────────────────────────────────

    public function testPipelineStatus(): void
    {
        $mock = new MockHttpHelper('{"is_busy":true,"pending_tasks":5}');
        $svc = new PipelineService($mock);
        $result = $svc->status();
        $this->assertTrue($result['is_busy']);
    }

    public function testPipelineQueueMetrics(): void
    {
        $mock = new MockHttpHelper('{"queue_depth":10}');
        $svc = new PipelineService($mock);
        $result = $svc->queueMetrics();
        $this->assertSame(10, $result['queue_depth']);
    }

    public function testPipelineStatusError(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{}', 500);
        $svc = new PipelineService($mock);
        $this->expectException(ApiError::class);
        $svc->status();
    }

    // ── Model Service ──────────────────────────────────────────────

    public function testModelCatalog(): void
    {
        $mock = new MockHttpHelper('{"providers":[{"name":"openai"}]}');
        $svc = new ModelService($mock);
        $result = $svc->catalog();
        $this->assertCount(1, $result['providers']);
    }

    public function testModelProviderStatus(): void
    {
        $mock = new MockHttpHelper('{"current_provider":"ollama"}');
        $svc = new ModelService($mock);
        $result = $svc->providerStatus();
        $this->assertSame('ollama', $result['current_provider']);
    }

    public function testModelHealth(): void
    {
        $mock = new MockHttpHelper('{"status":"ok","models":["qwen2.5"]}');
        $svc = new ModelService($mock);
        $result = $svc->health();
        $this->assertSame('ok', $result['status']);
        $this->assertStringContainsString('/api/v1/models/health', $mock->lastCall()['path']);
    }

    public function testModelCatalogError(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{}', 500);
        $svc = new ModelService($mock);
        $this->expectException(ApiError::class);
        $svc->catalog();
    }

    // ── Cost Service ───────────────────────────────────────────────

    public function testCostsSummary(): void
    {
        $mock = new MockHttpHelper('{"total_cost_usd":12.5}');
        $svc = new CostService($mock);
        $result = $svc->summary();
        $this->assertSame(12.5, $result['total_cost_usd']);
    }

    public function testCostsSummaryError(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{}', 500);
        $svc = new CostService($mock);
        $this->expectException(ApiError::class);
        $svc->summary();
    }

    // ── MockHttpHelper Tests ───────────────────────────────────────

    public function testMockTracksAllCalls(): void
    {
        $mock = new MockHttpHelper('{}');
        $svc = new HealthService($mock);
        $svc->check();
        $svc->check();
        $this->assertCount(2, $mock->calls);
    }

    public function testMockErrorIncludesStatusCode(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{"error":"not found"}', 404);
        try {
            $svc = new HealthService($mock);
            $svc->check();
            $this->fail('Expected ApiError');
        } catch (ApiError $e) {
            $this->assertSame(404, $e->statusCode);
        }
    }

    // ── Edge Case Tests ────────────────────────────────────────────

    public function testConfigBaseUrlTrailingSlash(): void
    {
        // Config stores exactly what is given; HttpHelper.requestRaw strips trailing slash
        $config = new Config(baseUrl: 'https://api.example.com/');
        $this->assertSame('https://api.example.com/', $config->baseUrl);
    }

    public function testDocumentUploadTextDefaults(): void
    {
        $mock = new MockHttpHelper('{"id":"d1"}');
        $svc = new DocumentService($mock);
        $svc->uploadText('Test', 'body');
        $this->assertSame('txt', $mock->lastCall()['body']['file_type']);
    }

    public function testDocumentsPagination(): void
    {
        $mock = new MockHttpHelper('{"documents":[]}');
        $svc = new DocumentService($mock);
        $svc->list(3, 50);
        $this->assertStringContainsString('page=3', $mock->lastCall()['path']);
        $this->assertStringContainsString('page_size=50', $mock->lastCall()['path']);
    }

    public function testEntityCreateBody(): void
    {
        $mock = new MockHttpHelper('{"status":"ok"}');
        $svc = new EntityService($mock);
        $svc->create('NODE', 'concept', 'A concept', 'src-1');
        $body = $mock->lastCall()['body'];
        $this->assertSame('NODE', $body['entity_name']);
        $this->assertSame('concept', $body['entity_type']);
        $this->assertSame('A concept', $body['description']);
        $this->assertSame('src-1', $body['source_id']);
    }

    public function testQueryExecuteWithMode(): void
    {
        $mock = new MockHttpHelper('{"answer":"yes"}');
        $svc = new QueryService($mock);
        $svc->execute('test', 'local');
        $this->assertSame('local', $mock->lastCall()['body']['mode']);
    }

    public function testChatCompletionsBody(): void
    {
        $mock = new MockHttpHelper('{"choices":[]}');
        $svc = new ChatService($mock);
        $svc->completions('hi', 'global', true);
        $body = $mock->lastCall()['body'];
        $this->assertSame('hi', $body['message']);
        $this->assertSame('global', $body['mode']);
        $this->assertTrue($body['stream']);
    }

    public function testApiErrorWithAllFields(): void
    {
        $err = new ApiError('HTTP 503: Service Unavailable', statusCode: 503, responseBody: '{"error":"overloaded"}');
        $this->assertSame(503, $err->statusCode);
        $this->assertStringContainsString('overloaded', $err->responseBody);
        $this->assertStringContainsString('503', $err->getMessage());
    }

    public function testModelHealthError(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{}', 502);
        $svc = new ModelService($mock);
        $this->expectException(ApiError::class);
        $svc->health();
    }

    public function testPipelineQueueMetricsError(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{}', 500);
        $svc = new PipelineService($mock);
        $this->expectException(ApiError::class);
        $svc->queueMetrics();
    }

    public function testGraphSearchError(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{}', 500);
        $svc = new GraphService($mock);
        $this->expectException(ApiError::class);
        $svc->search('test');
    }

    public function testEntityDeleteUrl(): void
    {
        $mock = new MockHttpHelper('{}');
        $svc = new EntityService($mock);
        $svc->delete('BOB');
        $this->assertStringContainsString('/api/v1/graph/entities/BOB', $mock->lastCall()['path']);
    }

    public function testDocumentDeleteUrl(): void
    {
        $mock = new MockHttpHelper('{}');
        $svc = new DocumentService($mock);
        $svc->delete('doc-abc');
        $this->assertStringContainsString('/api/v1/documents/doc-abc', $mock->lastCall()['path']);
    }

    public function testRelationshipsDefaultPagination(): void
    {
        $mock = new MockHttpHelper('{"items":[]}');
        $svc = new RelationshipService($mock);
        $svc->list();
        $this->assertStringContainsString('page=1', $mock->lastCall()['path']);
        $this->assertStringContainsString('page_size=20', $mock->lastCall()['path']);
    }

    public function testMockWillReturnChaining(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{"a":1}', 200);
        $svc = new HealthService($mock);
        $result = $svc->check();
        $this->assertSame(1, $result['a']);
    }

    // ── Conversation Service ───────────────────────────────────────

    public function testConversationsList(): void
    {
        $mock = new MockHttpHelper('{"conversations":[{"id":"c1","title":"Test"}]}');
        $svc = new ConversationService($mock);
        $result = $svc->list();
        $this->assertCount(1, $result['conversations']);
        $this->assertSame('GET', $mock->lastCall()['method']);
        $this->assertSame('/api/v1/conversations', $mock->lastCall()['path']);
    }

    public function testConversationsCreate(): void
    {
        $mock = new MockHttpHelper('{"id":"c2","title":"New Chat"}');
        $svc = new ConversationService($mock);
        $result = $svc->create('New Chat');
        $this->assertSame('c2', $result['id']);
        $this->assertSame('POST', $mock->lastCall()['method']);
        $this->assertSame('New Chat', $mock->lastCall()['body']['title']);
    }

    public function testConversationsCreateWithMode(): void
    {
        $mock = new MockHttpHelper('{"id":"c3","title":"Global Chat","mode":"global"}');
        $svc = new ConversationService($mock);
        $result = $svc->create('Global Chat', 'global');
        $this->assertSame('global', $mock->lastCall()['body']['mode']);
        $this->assertArrayNotHasKey('folder_id', $mock->lastCall()['body']);
    }

    public function testConversationsCreateWithFolder(): void
    {
        $mock = new MockHttpHelper('{"id":"c4","title":"Folder Chat"}');
        $svc = new ConversationService($mock);
        $svc->create('Folder Chat', null, 'folder-1');
        $this->assertSame('folder-1', $mock->lastCall()['body']['folder_id']);
        $this->assertArrayNotHasKey('mode', $mock->lastCall()['body']);
    }

    public function testConversationsCreateWithAllOptions(): void
    {
        $mock = new MockHttpHelper('{"id":"c5"}');
        $svc = new ConversationService($mock);
        $svc->create('Full Chat', 'hybrid', 'f-2');
        $body = $mock->lastCall()['body'];
        $this->assertSame('Full Chat', $body['title']);
        $this->assertSame('hybrid', $body['mode']);
        $this->assertSame('f-2', $body['folder_id']);
    }

    public function testConversationsListError(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{}', 500);
        $svc = new ConversationService($mock);
        $this->expectException(ApiError::class);
        $svc->list();
    }

    public function testConversationsCreateError(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{}', 422);
        $svc = new ConversationService($mock);
        $this->expectException(ApiError::class);
        $svc->create('Bad');
    }

    // ── Folder Service ─────────────────────────────────────────────

    public function testFoldersList(): void
    {
        $mock = new MockHttpHelper('[{"id":"f1","name":"Research"}]');
        $svc = new FolderService($mock);
        $result = $svc->list();
        $this->assertCount(1, $result);
        $this->assertSame('GET', $mock->lastCall()['method']);
    }

    public function testFoldersCreate(): void
    {
        $mock = new MockHttpHelper('{"id":"f2","name":"New Folder"}');
        $svc = new FolderService($mock);
        $result = $svc->create('New Folder');
        $this->assertSame('f2', $result['id']);
        $this->assertSame('POST', $mock->lastCall()['method']);
        $this->assertSame('New Folder', $mock->lastCall()['body']['name']);
    }

    public function testFoldersListError(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{}', 500);
        $svc = new FolderService($mock);
        $this->expectException(ApiError::class);
        $svc->list();
    }

    public function testFoldersCreateError(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{}', 409);
        $svc = new FolderService($mock);
        $this->expectException(ApiError::class);
        $svc->create('Duplicate');
    }

    // ── Additional Edge Case Tests ─────────────────────────────────

    public function testHealthCheckUrl(): void
    {
        $mock = new MockHttpHelper('{"status":"healthy"}');
        $svc = new HealthService($mock);
        $svc->check();
        $this->assertSame('/health', $mock->lastCall()['path']);
    }

    public function testTasksListUrl(): void
    {
        $mock = new MockHttpHelper('{"tasks":[]}');
        $svc = new TaskService($mock);
        $svc->list();
        $this->assertSame('/api/v1/tasks', $mock->lastCall()['path']);
    }

    public function testApiKeysListUrl(): void
    {
        $mock = new MockHttpHelper('[]');
        $svc = new ApiKeyService($mock);
        $svc->list();
        $this->assertSame('/api/v1/api-keys', $mock->lastCall()['path']);
    }

    public function testUsersListUrl(): void
    {
        $mock = new MockHttpHelper('[]');
        $svc = new UserService($mock);
        $svc->list();
        $this->assertSame('/api/v1/users', $mock->lastCall()['path']);
    }

    public function testTenantsListUrl(): void
    {
        $mock = new MockHttpHelper('{"items":[]}');
        $svc = new TenantService($mock);
        $svc->list();
        $this->assertSame('/api/v1/tenants', $mock->lastCall()['path']);
    }

    public function testCostsSummaryUrl(): void
    {
        $mock = new MockHttpHelper('{"total_cost_usd":0}');
        $svc = new CostService($mock);
        $svc->summary();
        $this->assertSame('/api/v1/costs/summary', $mock->lastCall()['path']);
    }

    public function testPipelineStatusUrl(): void
    {
        $mock = new MockHttpHelper('{"is_busy":false}');
        $svc = new PipelineService($mock);
        $svc->status();
        $this->assertSame('/api/v1/pipeline/status', $mock->lastCall()['path']);
    }

    public function testPipelineQueueMetricsUrl(): void
    {
        $mock = new MockHttpHelper('{"queue_depth":0}');
        $svc = new PipelineService($mock);
        $svc->queueMetrics();
        $this->assertSame('/api/v1/pipeline/queue-metrics', $mock->lastCall()['path']);
    }

    public function testModelCatalogUrl(): void
    {
        $mock = new MockHttpHelper('{"providers":[]}');
        $svc = new ModelService($mock);
        $svc->catalog();
        $this->assertSame('/api/v1/models', $mock->lastCall()['path']);
    }

    public function testModelProviderStatusUrl(): void
    {
        $mock = new MockHttpHelper('{"current_provider":"mock"}');
        $svc = new ModelService($mock);
        $svc->providerStatus();
        $this->assertSame('/api/v1/settings/provider/status', $mock->lastCall()['path']);
    }

    public function testDocumentsGetUrl(): void
    {
        $mock = new MockHttpHelper('{"id":"abc"}');
        $svc = new DocumentService($mock);
        $svc->get('abc');
        $this->assertStringContainsString('/api/v1/documents/abc', $mock->lastCall()['path']);
    }

    public function testEntitiesGetUrl(): void
    {
        $mock = new MockHttpHelper('{"entity_name":"FOO"}');
        $svc = new EntityService($mock);
        $svc->get('FOO');
        $this->assertStringContainsString('/api/v1/graph/entities/FOO', $mock->lastCall()['path']);
    }

    public function testQueryDefaultMode(): void
    {
        $mock = new MockHttpHelper('{"answer":"x"}');
        $svc = new QueryService($mock);
        $svc->execute('test');
        $this->assertSame('hybrid', $mock->lastCall()['body']['mode']);
    }

    public function testChatDefaultStream(): void
    {
        $mock = new MockHttpHelper('{"choices":[]}');
        $svc = new ChatService($mock);
        $svc->completions('hello');
        $this->assertFalse($mock->lastCall()['body']['stream']);
    }

    public function testChatStreamEnabled(): void
    {
        $mock = new MockHttpHelper('{"choices":[]}');
        $svc = new ChatService($mock);
        $svc->completions('hello', 'local', true);
        $this->assertTrue($mock->lastCall()['body']['stream']);
    }

    public function testEntityPagination(): void
    {
        $mock = new MockHttpHelper('{"items":[]}');
        $svc = new EntityService($mock);
        $svc->list(5, 100);
        $this->assertStringContainsString('page=5', $mock->lastCall()['path']);
        $this->assertStringContainsString('page_size=100', $mock->lastCall()['path']);
    }

    public function testClientHasConversationService(): void
    {
        $client = new Client();
        $this->assertInstanceOf(ConversationService::class, $client->conversations);
    }

    public function testClientHasFolderService(): void
    {
        $client = new Client();
        $this->assertInstanceOf(FolderService::class, $client->folders);
    }

    // ── Lineage Service ────────────────────────────────────────────

    public function testClientHasLineageService(): void
    {
        $client = new Client();
        $this->assertInstanceOf(LineageService::class, $client->lineage);
    }

    public function testEntityLineage(): void
    {
        $mock = new MockHttpHelper('{"entity_name":"ALICE","entity_type":"person","description_history":[]}');
        $svc = new LineageService($mock);
        $result = $svc->entityLineage('ALICE');
        $this->assertSame('ALICE', $result['entity_name']);
        $this->assertSame('person', $result['entity_type']);
        $this->assertIsArray($result['description_history']);
        $this->assertSame('GET', $mock->lastCall()['method']);
        $this->assertSame('/api/v1/lineage/entities/ALICE', $mock->lastCall()['path']);
    }

    public function testEntityLineageUrlEncoding(): void
    {
        $mock = new MockHttpHelper('{"entity_name":"HELLO WORLD"}');
        $svc = new LineageService($mock);
        $svc->entityLineage('HELLO WORLD');
        $this->assertSame('/api/v1/lineage/entities/HELLO%20WORLD', $mock->lastCall()['path']);
    }

    public function testEntityLineageSpecialChars(): void
    {
        $mock = new MockHttpHelper('{"entity_name":"O\'BRIEN"}');
        $svc = new LineageService($mock);
        $svc->entityLineage("O'BRIEN");
        $this->assertStringContainsString('/api/v1/lineage/entities/', $mock->lastCall()['path']);
        $this->assertSame('GET', $mock->lastCall()['method']);
    }

    public function testDocumentLineage(): void
    {
        $mock = new MockHttpHelper('{"document_id":"d1","entities":[],"relationships":[]}');
        $svc = new LineageService($mock);
        $result = $svc->documentLineage('d1');
        $this->assertSame('d1', $result['document_id']);
        $this->assertIsArray($result['entities']);
        $this->assertIsArray($result['relationships']);
        $this->assertSame('GET', $mock->lastCall()['method']);
        $this->assertSame('/api/v1/lineage/documents/d1', $mock->lastCall()['path']);
    }

    public function testDocumentLineageEmpty(): void
    {
        $mock = new MockHttpHelper('{"document_id":"d2","entities":[],"relationships":[],"extraction_stats":null}');
        $svc = new LineageService($mock);
        $result = $svc->documentLineage('d2');
        $this->assertSame('d2', $result['document_id']);
        $this->assertEmpty($result['entities']);
        $this->assertNull($result['extraction_stats']);
    }

    public function testDocumentFullLineage(): void
    {
        $mock = new MockHttpHelper('{"document_id":"d1","chunks":[],"total_chunks":5}');
        $svc = new LineageService($mock);
        $result = $svc->documentFullLineage('d1');
        $this->assertSame('d1', $result['document_id']);
        $this->assertIsArray($result['chunks']);
        $this->assertSame(5, $result['total_chunks']);
        $this->assertSame('GET', $mock->lastCall()['method']);
        $this->assertSame('/api/v1/documents/d1/lineage', $mock->lastCall()['path']);
    }

    public function testExportLineageJson(): void
    {
        $mock = new MockHttpHelper('{"document_id":"d1","format":"json"}');
        $svc = new LineageService($mock);
        $result = $svc->exportLineage('d1');
        $this->assertIsString($result);
        $this->assertStringContainsString('document_id', $result);
        $this->assertSame('GET', $mock->lastCall()['method']);
        $this->assertSame('/api/v1/documents/d1/lineage/export?format=json', $mock->lastCall()['path']);
    }

    public function testExportLineageCsv(): void
    {
        $mock = new MockHttpHelper("entity_name,entity_type\nALICE,person");
        $svc = new LineageService($mock);
        $result = $svc->exportLineage('d1', 'csv');
        $this->assertIsString($result);
        $this->assertStringContainsString('ALICE', $result);
        $this->assertSame('/api/v1/documents/d1/lineage/export?format=csv', $mock->lastCall()['path']);
    }

    public function testChunkDetail(): void
    {
        $mock = new MockHttpHelper('{"chunk_id":"c1","content":"hello","entities":[],"relationships":[]}');
        $svc = new LineageService($mock);
        $result = $svc->chunkDetail('c1');
        $this->assertSame('c1', $result['chunk_id']);
        $this->assertSame('hello', $result['content']);
        $this->assertIsArray($result['entities']);
        $this->assertSame('GET', $mock->lastCall()['method']);
        $this->assertSame('/api/v1/chunks/c1', $mock->lastCall()['path']);
    }

    public function testChunkDetailMinimal(): void
    {
        $mock = new MockHttpHelper('{"chunk_id":"c2","content":""}');
        $svc = new LineageService($mock);
        $result = $svc->chunkDetail('c2');
        $this->assertSame('c2', $result['chunk_id']);
        $this->assertSame('', $result['content']);
    }

    public function testChunkLineage(): void
    {
        $mock = new MockHttpHelper('{"chunk_id":"c1","document_id":"d1","entities":[],"relationships":[]}');
        $svc = new LineageService($mock);
        $result = $svc->chunkLineage('c1');
        $this->assertSame('c1', $result['chunk_id']);
        $this->assertSame('d1', $result['document_id']);
        $this->assertSame('GET', $mock->lastCall()['method']);
        $this->assertSame('/api/v1/chunks/c1/lineage', $mock->lastCall()['path']);
    }

    public function testEntityProvenance(): void
    {
        $mock = new MockHttpHelper('{"entity_name":"BOB","source_documents":[],"related_entities":[]}');
        $svc = new LineageService($mock);
        $result = $svc->entityProvenance('ent-1');
        $this->assertSame('BOB', $result['entity_name']);
        $this->assertIsArray($result['source_documents']);
        $this->assertIsArray($result['related_entities']);
        $this->assertSame('GET', $mock->lastCall()['method']);
        $this->assertSame('/api/v1/entities/ent-1/provenance', $mock->lastCall()['path']);
    }

    public function testEntityProvenanceMinimal(): void
    {
        $mock = new MockHttpHelper('{"entity_name":"X"}');
        $svc = new LineageService($mock);
        $result = $svc->entityProvenance('ent-2');
        $this->assertSame('X', $result['entity_name']);
        $this->assertSame('/api/v1/entities/ent-2/provenance', $mock->lastCall()['path']);
    }

    public function testLineageErrorHandling(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{"error":"Not Found"}', 404);
        $svc = new LineageService($mock);
        $this->expectException(ApiError::class);
        $svc->entityLineage('MISSING');
    }

    // ── OODA-33: Health Service Additional Tests ──────────────────

    public function testHealthReady(): void
    {
        $mock = new MockHttpHelper('{"status":"ready"}');
        $svc = new HealthService($mock);
        $result = $svc->ready();
        $this->assertSame('ready', $result['status']);
        $this->assertSame('/ready', $mock->lastCall()['path']);
    }

    public function testHealthLive(): void
    {
        $mock = new MockHttpHelper('{"status":"live"}');
        $svc = new HealthService($mock);
        $result = $svc->live();
        $this->assertSame('live', $result['status']);
        $this->assertSame('/live', $mock->lastCall()['path']);
    }

    public function testHealthMetrics(): void
    {
        $mock = new MockHttpHelper("# HELP process_cpu\n# TYPE process_cpu gauge\nprocess_cpu 0.5");
        $svc = new HealthService($mock);
        $result = $svc->metrics();
        $this->assertIsString($result);
        $this->assertStringContainsString('process_cpu', $result);
        $this->assertSame('/metrics', $mock->lastCall()['path']);
    }

    // ── OODA-33: Auth Service Tests ───────────────────────────────

    public function testAuthLogin(): void
    {
        $mock = new MockHttpHelper('{"token":"jwt123","refresh_token":"rf456"}');
        $svc = new AuthService($mock);
        $result = $svc->login('admin', 'secret');
        $this->assertSame('jwt123', $result['token']);
        $this->assertSame('POST', $mock->lastCall()['method']);
        $this->assertSame('/api/v1/auth/login', $mock->lastCall()['path']);
        $this->assertSame('admin', $mock->lastCall()['body']['username']);
    }

    public function testAuthLogout(): void
    {
        $mock = new MockHttpHelper('{"status":"logged_out"}');
        $svc = new AuthService($mock);
        $result = $svc->logout();
        $this->assertSame('logged_out', $result['status']);
        $this->assertSame('POST', $mock->lastCall()['method']);
        $this->assertSame('/api/v1/auth/logout', $mock->lastCall()['path']);
    }

    public function testAuthRefresh(): void
    {
        $mock = new MockHttpHelper('{"token":"newjwt","refresh_token":"newrf"}');
        $svc = new AuthService($mock);
        $result = $svc->refresh('old-rf');
        $this->assertSame('newjwt', $result['token']);
        $this->assertSame('old-rf', $mock->lastCall()['body']['refresh_token']);
    }

    public function testAuthMe(): void
    {
        $mock = new MockHttpHelper('{"id":"u1","username":"admin","email":"admin@test.com"}');
        $svc = new AuthService($mock);
        $result = $svc->me();
        $this->assertSame('admin', $result['username']);
        $this->assertSame('GET', $mock->lastCall()['method']);
        $this->assertSame('/api/v1/auth/me', $mock->lastCall()['path']);
    }

    public function testAuthLoginError(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{"error":"invalid credentials"}', 401);
        $svc = new AuthService($mock);
        $this->expectException(ApiError::class);
        $svc->login('bad', 'creds');
    }

    // ── OODA-33: Document Service Additional Tests ────────────────

    public function testDocumentDeleteAll(): void
    {
        $mock = new MockHttpHelper('{"deleted_count":5}');
        $svc = new DocumentService($mock);
        $result = $svc->deleteAll();
        $this->assertSame(5, $result['deleted_count']);
        $this->assertSame('DELETE', $mock->lastCall()['method']);
        $this->assertSame('/api/v1/documents', $mock->lastCall()['path']);
    }

    public function testDocumentReprocess(): void
    {
        $mock = new MockHttpHelper('{"reprocessed_count":3}');
        $svc = new DocumentService($mock);
        $result = $svc->reprocess();
        $this->assertSame(3, $result['reprocessed_count']);
        $this->assertSame('POST', $mock->lastCall()['method']);
        $this->assertSame('/api/v1/documents/reprocess', $mock->lastCall()['path']);
    }

    public function testDocumentRecoverStuck(): void
    {
        $mock = new MockHttpHelper('{"recovered_count":2}');
        $svc = new DocumentService($mock);
        $result = $svc->recoverStuck();
        $this->assertSame(2, $result['recovered_count']);
        $this->assertSame('/api/v1/documents/recover-stuck', $mock->lastCall()['path']);
    }

    public function testDocumentRetryChunks(): void
    {
        $mock = new MockHttpHelper('{"retried_count":1}');
        $svc = new DocumentService($mock);
        $result = $svc->retryChunks('doc-1');
        $this->assertSame(1, $result['retried_count']);
        $this->assertSame('/api/v1/documents/doc-1/retry-chunks', $mock->lastCall()['path']);
    }

    public function testDocumentFailedChunks(): void
    {
        $mock = new MockHttpHelper('{"chunks":[{"id":"c1","error":"parse failed"}]}');
        $svc = new DocumentService($mock);
        $result = $svc->failedChunks('doc-1');
        $this->assertCount(1, $result['chunks']);
        $this->assertSame('/api/v1/documents/doc-1/failed-chunks', $mock->lastCall()['path']);
    }

    // ── OODA-33: Entity Service Additional Tests ──────────────────

    public function testEntityUpdate(): void
    {
        $mock = new MockHttpHelper('{"entity_name":"ALICE","entity_type":"updated"}');
        $svc = new EntityService($mock);
        $result = $svc->update('ALICE', ['entity_type' => 'updated']);
        $this->assertSame('updated', $result['entity_type']);
        $this->assertSame('PUT', $mock->lastCall()['method']);
        $this->assertStringContainsString('/api/v1/graph/entities/ALICE', $mock->lastCall()['path']);
    }

    public function testEntityMerge(): void
    {
        $mock = new MockHttpHelper('{"status":"merged","target_name":"BOB"}');
        $svc = new EntityService($mock);
        $result = $svc->merge('ALICE', 'BOB');
        $this->assertSame('merged', $result['status']);
        $this->assertSame('BOB', $mock->lastCall()['body']['target_name']);
        $this->assertSame('/api/v1/graph/entities/merge', $mock->lastCall()['path']);
    }

    public function testEntityNeighborhood(): void
    {
        $mock = new MockHttpHelper('{"nodes":[{"id":"n1"}],"edges":[]}');
        $svc = new EntityService($mock);
        $result = $svc->neighborhood('ALICE', 2);
        $this->assertCount(1, $result['nodes']);
        $this->assertStringContainsString('depth=2', $mock->lastCall()['path']);
    }

    // ── OODA-33: Relationship Service Additional Tests ────────────

    public function testRelationshipGet(): void
    {
        $mock = new MockHttpHelper('{"id":"r1","source":"A","target":"B"}');
        $svc = new RelationshipService($mock);
        $result = $svc->get('r1');
        $this->assertSame('A', $result['source']);
        $this->assertStringContainsString('/api/v1/graph/relationships/r1', $mock->lastCall()['path']);
    }

    public function testRelationshipUpdate(): void
    {
        $mock = new MockHttpHelper('{"id":"r1","weight":0.9}');
        $svc = new RelationshipService($mock);
        $result = $svc->update('r1', ['weight' => 0.9]);
        $this->assertSame(0.9, $result['weight']);
        $this->assertSame('PUT', $mock->lastCall()['method']);
    }

    public function testRelationshipDelete(): void
    {
        $mock = new MockHttpHelper('{"status":"deleted"}');
        $svc = new RelationshipService($mock);
        $svc->delete('r1');
        $this->assertSame('DELETE', $mock->lastCall()['method']);
    }

    // ── OODA-33: Graph Service Additional Tests ───────────────────

    public function testGraphGetNode(): void
    {
        $mock = new MockHttpHelper('{"id":"n1","label":"ALICE","type":"person"}');
        $svc = new GraphService($mock);
        $result = $svc->getNode('n1');
        $this->assertSame('ALICE', $result['label']);
        $this->assertStringContainsString('/api/v1/graph/nodes/n1', $mock->lastCall()['path']);
    }

    public function testGraphSearchLabels(): void
    {
        $mock = new MockHttpHelper('{"labels":[{"name":"person","count":10}]}');
        $svc = new GraphService($mock);
        $result = $svc->searchLabels('per');
        $this->assertCount(1, $result['labels']);
        $this->assertStringContainsString('q=per', $mock->lastCall()['path']);
    }

    public function testGraphPopularLabels(): void
    {
        $mock = new MockHttpHelper('{"labels":[{"name":"person","count":100}]}');
        $svc = new GraphService($mock);
        $result = $svc->popularLabels(10);
        $this->assertCount(1, $result['labels']);
        $this->assertStringContainsString('limit=10', $mock->lastCall()['path']);
    }

    public function testGraphDegreesBatch(): void
    {
        $mock = new MockHttpHelper('{"degrees":[{"node_id":"n1","degree":5}]}');
        $svc = new GraphService($mock);
        $result = $svc->degreesBatch(['n1', 'n2']);
        $this->assertCount(1, $result['degrees']);
        $this->assertSame('POST', $mock->lastCall()['method']);
    }

    // ── OODA-33: Tenant Service Additional Tests ──────────────────

    public function testTenantGet(): void
    {
        $mock = new MockHttpHelper('{"id":"t1","name":"Acme"}');
        $svc = new TenantService($mock);
        $result = $svc->get('t1');
        $this->assertSame('Acme', $result['name']);
    }

    public function testTenantCreate(): void
    {
        $mock = new MockHttpHelper('{"id":"t2","name":"NewCo"}');
        $svc = new TenantService($mock);
        $result = $svc->create('NewCo');
        $this->assertSame('NewCo', $mock->lastCall()['body']['name']);
    }

    public function testTenantUpdate(): void
    {
        $mock = new MockHttpHelper('{"id":"t1","name":"UpdatedCo"}');
        $svc = new TenantService($mock);
        $result = $svc->update('t1', ['name' => 'UpdatedCo']);
        $this->assertSame('PUT', $mock->lastCall()['method']);
    }

    public function testTenantDelete(): void
    {
        $mock = new MockHttpHelper('{"status":"deleted"}');
        $svc = new TenantService($mock);
        $svc->delete('t1');
        $this->assertSame('DELETE', $mock->lastCall()['method']);
    }

    // ── OODA-33: User Service Additional Tests ────────────────────

    public function testUserGet(): void
    {
        $mock = new MockHttpHelper('{"id":"u1","username":"alice"}');
        $svc = new UserService($mock);
        $result = $svc->get('u1');
        $this->assertSame('alice', $result['username']);
    }

    public function testUserCreate(): void
    {
        $mock = new MockHttpHelper('{"id":"u2","username":"bob"}');
        $svc = new UserService($mock);
        $result = $svc->create('bob', 'bob@test.com');
        $this->assertSame('bob', $mock->lastCall()['body']['username']);
        $this->assertSame('bob@test.com', $mock->lastCall()['body']['email']);
    }

    public function testUserDelete(): void
    {
        $mock = new MockHttpHelper('{"status":"deleted"}');
        $svc = new UserService($mock);
        $svc->delete('u1');
        $this->assertSame('DELETE', $mock->lastCall()['method']);
    }

    // ── OODA-33: API Key Service Additional Tests ─────────────────

    public function testApiKeyCreate(): void
    {
        $mock = new MockHttpHelper('{"id":"ak-1","key":"sk-test123"}');
        $svc = new ApiKeyService($mock);
        $result = $svc->create('Production');
        $this->assertSame('Production', $mock->lastCall()['body']['name']);
    }

    public function testApiKeyDelete(): void
    {
        $mock = new MockHttpHelper('{"status":"deleted"}');
        $svc = new ApiKeyService($mock);
        $svc->delete('ak-1');
        $this->assertStringContainsString('/api/v1/api-keys/ak-1', $mock->lastCall()['path']);
    }

    public function testApiKeyRevoke(): void
    {
        $mock = new MockHttpHelper('{"status":"revoked"}');
        $svc = new ApiKeyService($mock);
        $result = $svc->revoke('ak-1');
        $this->assertSame('revoked', $result['status']);
        $this->assertSame('DELETE', $mock->lastCall()['method']);
        $this->assertStringContainsString('/api/v1/api-keys/ak-1', $mock->lastCall()['path']);
        $this->assertStringNotContainsString('/revoke', $mock->lastCall()['path']);
    }

    // ── OODA-33: Task Service Additional Tests ────────────────────

    public function testTaskGet(): void
    {
        $mock = new MockHttpHelper('{"track_id":"t1","status":"running"}');
        $svc = new TaskService($mock);
        $result = $svc->get('t1');
        $this->assertSame('running', $result['status']);
    }

    public function testTaskCancel(): void
    {
        $mock = new MockHttpHelper('{"status":"cancelled"}');
        $svc = new TaskService($mock);
        $result = $svc->cancel('t1');
        $this->assertSame('cancelled', $result['status']);
        $this->assertStringContainsString('/cancel', $mock->lastCall()['path']);
    }

    public function testTaskRetry(): void
    {
        $mock = new MockHttpHelper('{"status":"retrying"}');
        $svc = new TaskService($mock);
        $result = $svc->retry('t1');
        $this->assertSame('retrying', $result['status']);
        $this->assertStringContainsString('/retry', $mock->lastCall()['path']);
    }

    // ── OODA-33: Pipeline Service Additional Tests ────────────────

    public function testPipelineCancel(): void
    {
        $mock = new MockHttpHelper('{"status":"cancelled"}');
        $svc = new PipelineService($mock);
        $result = $svc->cancel();
        $this->assertSame('cancelled', $result['status']);
        $this->assertSame('/api/v1/pipeline/cancel', $mock->lastCall()['path']);
    }

    // ── OODA-33: Model Service Additional Tests ───────────────────

    public function testModelListLlm(): void
    {
        $mock = new MockHttpHelper('{"models":[{"id":"gpt-4","name":"GPT-4"}]}');
        $svc = new ModelService($mock);
        $result = $svc->listLlm();
        $this->assertCount(1, $result['models']);
        $this->assertSame('/api/v1/models/llm', $mock->lastCall()['path']);
    }

    public function testModelListEmbedding(): void
    {
        $mock = new MockHttpHelper('{"models":[{"id":"text-embedding-3-small"}]}');
        $svc = new ModelService($mock);
        $result = $svc->listEmbedding();
        $this->assertCount(1, $result['models']);
        $this->assertSame('/api/v1/models/embedding', $mock->lastCall()['path']);
    }

    public function testModelGetProvider(): void
    {
        $mock = new MockHttpHelper('{"name":"openai","models":["gpt-4"]}');
        $svc = new ModelService($mock);
        $result = $svc->getProvider('openai');
        $this->assertSame('openai', $result['name']);
        $this->assertStringContainsString('/api/v1/models/openai', $mock->lastCall()['path']);
    }

    public function testModelGetModel(): void
    {
        $mock = new MockHttpHelper('{"id":"gpt-4","context_window":128000}');
        $svc = new ModelService($mock);
        $result = $svc->getModel('openai', 'gpt-4');
        $this->assertSame(128000, $result['context_window']);
        $this->assertStringContainsString('/api/v1/models/openai/gpt-4', $mock->lastCall()['path']);
    }

    public function testModelListProviders(): void
    {
        $mock = new MockHttpHelper('{"providers":["openai","ollama"]}');
        $svc = new ModelService($mock);
        $result = $svc->listProviders();
        $this->assertCount(2, $result['providers']);
        $this->assertSame('/api/v1/settings/providers', $mock->lastCall()['path']);
    }

    // ── OODA-33: Cost Service Additional Tests ────────────────────

    public function testCostHistory(): void
    {
        $mock = new MockHttpHelper('{"entries":[{"date":"2026-01-15","cost_usd":10.5}]}');
        $svc = new CostService($mock);
        $result = $svc->history();
        $this->assertCount(1, $result['entries']);
        $this->assertSame('/api/v1/costs/history', $mock->lastCall()['path']);
    }

    public function testCostPricing(): void
    {
        $mock = new MockHttpHelper('{"models":[{"model":"gpt-4","input_cost_per_1k":0.03}]}');
        $svc = new CostService($mock);
        $result = $svc->pricing();
        $this->assertCount(1, $result['models']);
        $this->assertSame('/api/v1/pipeline/costs/pricing', $mock->lastCall()['path']);
    }

    public function testCostEstimate(): void
    {
        $mock = new MockHttpHelper('{"estimated_cost_usd":5.25}');
        $svc = new CostService($mock);
        $result = $svc->estimate(['document_count' => 100]);
        $this->assertSame(5.25, $result['estimated_cost_usd']);
        $this->assertSame('POST', $mock->lastCall()['method']);
    }

    public function testCostUpdateBudget(): void
    {
        $mock = new MockHttpHelper('{"monthly_budget_usd":100}');
        $svc = new CostService($mock);
        $result = $svc->updateBudget(['monthly_budget_usd' => 100]);
        $this->assertSame(100, $result['monthly_budget_usd']);
        $this->assertSame('PATCH', $mock->lastCall()['method']);
    }

    // ── OODA-33: Conversation Service Additional Tests ────────────

    public function testConversationGet(): void
    {
        $mock = new MockHttpHelper('{"id":"c1","title":"Test Chat"}');
        $svc = new ConversationService($mock);
        $result = $svc->get('c1');
        $this->assertSame('Test Chat', $result['title']);
    }

    public function testConversationUpdate(): void
    {
        $mock = new MockHttpHelper('{"id":"c1","title":"Updated"}');
        $svc = new ConversationService($mock);
        $result = $svc->update('c1', ['title' => 'Updated']);
        $this->assertSame('PATCH', $mock->lastCall()['method']);
    }

    public function testConversationDelete(): void
    {
        $mock = new MockHttpHelper('{"status":"deleted"}');
        $svc = new ConversationService($mock);
        $svc->delete('c1');
        $this->assertSame('DELETE', $mock->lastCall()['method']);
    }

    public function testConversationImport(): void
    {
        $mock = new MockHttpHelper('{"id":"c2","title":"Imported"}');
        $svc = new ConversationService($mock);
        $result = $svc->import(['title' => 'Imported', 'messages' => []]);
        $this->assertSame('Imported', $result['title']);
        $this->assertSame('/api/v1/conversations/import', $mock->lastCall()['path']);
    }

    public function testConversationShare(): void
    {
        $mock = new MockHttpHelper('{"share_id":"sh-123"}');
        $svc = new ConversationService($mock);
        $result = $svc->share('c1');
        $this->assertSame('sh-123', $result['share_id']);
        $this->assertStringContainsString('/share', $mock->lastCall()['path']);
    }

    public function testConversationUnshare(): void
    {
        $mock = new MockHttpHelper('{"status":"unshared"}');
        $svc = new ConversationService($mock);
        $result = $svc->unshare('c1');
        $this->assertSame('unshared', $result['status']);
        $this->assertSame('DELETE', $mock->lastCall()['method']);
    }

    public function testConversationListMessages(): void
    {
        $mock = new MockHttpHelper('{"messages":[{"id":"m1","content":"Hello"}]}');
        $svc = new ConversationService($mock);
        $result = $svc->listMessages('c1');
        $this->assertCount(1, $result['messages']);
        $this->assertStringContainsString('/messages', $mock->lastCall()['path']);
    }

    public function testConversationCreateMessage(): void
    {
        $mock = new MockHttpHelper('{"id":"m2","content":"Test","role":"user"}');
        $svc = new ConversationService($mock);
        $result = $svc->createMessage('c1', 'Test');
        $this->assertSame('Test', $mock->lastCall()['body']['content']);
        $this->assertSame('user', $mock->lastCall()['body']['role']);
    }

    public function testConversationBulkArchive(): void
    {
        $mock = new MockHttpHelper('{"archived_count":3}');
        $svc = new ConversationService($mock);
        $result = $svc->bulkArchive(['c1', 'c2', 'c3']);
        $this->assertSame(3, $result['archived_count']);
        $this->assertSame('/api/v1/conversations/bulk/archive', $mock->lastCall()['path']);
    }

    public function testConversationBulkMove(): void
    {
        $mock = new MockHttpHelper('{"moved_count":2}');
        $svc = new ConversationService($mock);
        $result = $svc->bulkMove(['c1', 'c2'], 'folder-1');
        $this->assertSame(2, $result['moved_count']);
        $this->assertSame('folder-1', $mock->lastCall()['body']['folder_id']);
    }

    // ── OODA-33: Folder Service Additional Tests ──────────────────

    public function testFolderGet(): void
    {
        $mock = new MockHttpHelper('{"id":"f1","name":"Research"}');
        $svc = new FolderService($mock);
        $result = $svc->get('f1');
        $this->assertSame('Research', $result['name']);
    }

    public function testFolderUpdate(): void
    {
        $mock = new MockHttpHelper('{"id":"f1","name":"Updated"}');
        $svc = new FolderService($mock);
        $result = $svc->update('f1', ['name' => 'Updated']);
        $this->assertSame('PATCH', $mock->lastCall()['method']);
    }

    public function testFolderDelete(): void
    {
        $mock = new MockHttpHelper('{"status":"deleted"}');
        $svc = new FolderService($mock);
        $svc->delete('f1');
        $this->assertSame('DELETE', $mock->lastCall()['method']);
    }

    // ── OODA-33: Workspace Service Tests ──────────────────────────

    public function testWorkspaceList(): void
    {
        $mock = new MockHttpHelper('{"workspaces":[{"id":"ws1","name":"Default"}]}');
        $svc = new WorkspaceService($mock);
        $result = $svc->list();
        $this->assertCount(1, $result['workspaces']);
        $this->assertSame('/api/v1/workspaces', $mock->lastCall()['path']);
    }

    public function testWorkspaceGet(): void
    {
        $mock = new MockHttpHelper('{"id":"ws1","name":"Default"}');
        $svc = new WorkspaceService($mock);
        $result = $svc->get('ws1');
        $this->assertSame('Default', $result['name']);
    }

    public function testWorkspaceCreate(): void
    {
        $mock = new MockHttpHelper('{"id":"ws2","name":"NewWs"}');
        $svc = new WorkspaceService($mock);
        $result = $svc->create('NewWs');
        $this->assertSame('NewWs', $mock->lastCall()['body']['name']);
    }

    public function testWorkspaceCreateWithTenant(): void
    {
        $mock = new MockHttpHelper('{"id":"ws3","name":"TenantWs"}');
        $svc = new WorkspaceService($mock);
        $svc->create('TenantWs', 't1');
        $this->assertSame('t1', $mock->lastCall()['body']['tenant_id']);
    }

    public function testWorkspaceUpdate(): void
    {
        $mock = new MockHttpHelper('{"id":"ws1","name":"Updated"}');
        $svc = new WorkspaceService($mock);
        $result = $svc->update('ws1', ['name' => 'Updated']);
        $this->assertSame('PUT', $mock->lastCall()['method']);
    }

    public function testWorkspaceDelete(): void
    {
        $mock = new MockHttpHelper('{"status":"deleted"}');
        $svc = new WorkspaceService($mock);
        $svc->delete('ws1');
        $this->assertSame('DELETE', $mock->lastCall()['method']);
    }

    public function testWorkspaceMetricsHistory(): void
    {
        $mock = new MockHttpHelper('{"metrics":[{"date":"2026-01-15","documents":100}]}');
        $svc = new WorkspaceService($mock);
        $result = $svc->metricsHistory('ws1');
        $this->assertCount(1, $result['metrics']);
        $this->assertStringContainsString('/metrics-history', $mock->lastCall()['path']);
    }

    public function testWorkspaceRebuildEmbeddings(): void
    {
        $mock = new MockHttpHelper('{"task_id":"task-1"}');
        $svc = new WorkspaceService($mock);
        $result = $svc->rebuildEmbeddings('ws1');
        $this->assertSame('task-1', $result['task_id']);
        $this->assertStringContainsString('/rebuild-embeddings', $mock->lastCall()['path']);
    }

    public function testWorkspaceRebuildKnowledgeGraph(): void
    {
        $mock = new MockHttpHelper('{"task_id":"task-2"}');
        $svc = new WorkspaceService($mock);
        $result = $svc->rebuildKnowledgeGraph('ws1');
        $this->assertSame('task-2', $result['task_id']);
        $this->assertStringContainsString('/rebuild-knowledge-graph', $mock->lastCall()['path']);
    }

    public function testWorkspaceReprocessDocuments(): void
    {
        $mock = new MockHttpHelper('{"task_id":"task-3"}');
        $svc = new WorkspaceService($mock);
        $result = $svc->reprocessDocuments('ws1');
        $this->assertSame('task-3', $result['task_id']);
        $this->assertStringContainsString('/reprocess-documents', $mock->lastCall()['path']);
    }

    public function testWorkspaceListError(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{}', 500);
        $svc = new WorkspaceService($mock);
        $this->expectException(ApiError::class);
        $svc->list();
    }

    // ── OODA-33: Shared Service Tests ─────────────────────────────

    public function testSharedGet(): void
    {
        $mock = new MockHttpHelper('{"id":"c1","title":"Shared Chat","messages":[]}');
        $svc = new SharedService($mock);
        $result = $svc->get('sh-123');
        $this->assertSame('Shared Chat', $result['title']);
        $this->assertSame('/api/v1/shared/sh-123', $mock->lastCall()['path']);
    }

    public function testSharedGetNotFound(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{"error":"not found"}', 404);
        $svc = new SharedService($mock);
        $this->expectException(ApiError::class);
        $svc->get('invalid');
    }

    public function testDocumentTrack(): void
    {
        $mock = new MockHttpHelper('{"track_id":"tk1","status":"processing"}');
        $svc = new DocumentService($mock);
        $result = $svc->track('tk1');
        $this->assertSame('tk1', $result['track_id']);
        $this->assertStringContainsString('/api/v1/documents/track/tk1', $mock->lastCall()['path']);
    }

    public function testDocumentGetMetadata(): void
    {
        $mock = new MockHttpHelper('{"metadata":{"author":"John","category":"research"}}');
        $svc = new DocumentService($mock);
        $result = $svc->getMetadata('d1');
        $this->assertSame('John', $result['metadata']['author']);
        $this->assertStringContainsString('/api/v1/documents/d1/metadata', $mock->lastCall()['path']);
    }

    public function testDocumentGetMetadataEmpty(): void
    {
        $mock = new MockHttpHelper('{"metadata":{}}');
        $svc = new DocumentService($mock);
        $result = $svc->getMetadata('d1');
        $this->assertEmpty($result['metadata']);
    }

    public function testDocumentPdfStatus(): void
    {
        $mock = new MockHttpHelper('{"id":"pdf-1","extraction_status":"completed","page_count":10}');
        $svc = new DocumentService($mock);
        $result = $svc->pdfStatus('pdf-1');
        $this->assertSame('completed', $result['extraction_status']);
        $this->assertStringContainsString('/api/v1/documents/pdf/pdf-1', $mock->lastCall()['path']);
        $this->assertStringNotContainsString('/status', $mock->lastCall()['path']);
    }

    public function testDocumentPdfProgress(): void
    {
        $mock = new MockHttpHelper('{"track_id":"trk-1","status":"processing"}');
        $svc = new DocumentService($mock);
        $result = $svc->pdfProgress('trk-1');
        $this->assertSame('trk-1', $result['track_id']);
        $this->assertStringContainsString('/api/v1/documents/pdf/progress/trk-1', $mock->lastCall()['path']);
    }

    public function testEntityExists(): void
    {
        $mock = new MockHttpHelper('{"exists":true,"entity_id":"E1","entity_type":"PERSON","degree":3}');
        $svc = new EntityService($mock);
        $result = $svc->exists('ALICE');
        $this->assertTrue($result['exists']);
        $this->assertStringContainsString('/api/v1/graph/entities/exists', $mock->lastCall()['path']);
        $this->assertStringContainsString('entity_name=ALICE', $mock->lastCall()['path']);
    }

    public function testRelationshipCreate(): void
    {
        $mock = new MockHttpHelper('{"status":"success","message":"ok","relationship":{"id":"rel-1"}}');
        $svc = new RelationshipService($mock);
        $result = $svc->create('A', 'B', 'works at', 'A works at B');
        $this->assertSame('success', $result['status']);
        $this->assertSame('POST', $mock->lastCall()['method']);
        $body = $mock->lastCall()['body'];
        $this->assertSame('A', $body['src_id']);
        $this->assertSame('B', $body['tgt_id']);
        $this->assertSame('works at', $body['keywords']);
    }

    // ── OODA-39: Chat Service - Additional Methods ────────────────

    public function testChatCompletionsWithConversation(): void
    {
        $mock = new MockHttpHelper('{"message":"Hello!","conversation_id":"c1"}');
        $svc = new ChatService($mock);
        $result = $svc->completionsWithConversation('c1', 'Hi there', 'hybrid');
        $this->assertSame('c1', $mock->lastCall()['body']['conversation_id']);
        $this->assertSame('Hi there', $mock->lastCall()['body']['message']);
        $this->assertSame('hybrid', $mock->lastCall()['body']['mode']);
    }

    public function testChatCompletionsWithConversationDefaultMode(): void
    {
        $mock = new MockHttpHelper('{"message":"Response"}');
        $svc = new ChatService($mock);
        $svc->completionsWithConversation('c1', 'Question');
        $this->assertSame('hybrid', $mock->lastCall()['body']['mode']);
    }

    // ── OODA-39: Conversation Service - Message Methods ───────────

    public function testConversationUpdateMessage(): void
    {
        $mock = new MockHttpHelper('{"id":"m1","content":"Updated content","role":"user"}');
        $svc = new ConversationService($mock);
        $result = $svc->updateMessage('c1', 'm1', 'Updated content');
        $this->assertSame('PATCH', $mock->lastCall()['method']);
        $this->assertStringContainsString('/api/v1/messages/m1', $mock->lastCall()['path']);
        $this->assertSame('Updated content', $mock->lastCall()['body']['content']);
    }

    public function testConversationUpdateMessagePreservesId(): void
    {
        $mock = new MockHttpHelper('{"id":"m1","content":"New text"}');
        $svc = new ConversationService($mock);
        $result = $svc->updateMessage('c1', 'm1', 'New text');
        $this->assertSame('m1', $result['id']);
    }

    public function testConversationDeleteMessage(): void
    {
        $mock = new MockHttpHelper('{"status":"deleted"}');
        $svc = new ConversationService($mock);
        $svc->deleteMessage('c1', 'm1');
        $this->assertSame('DELETE', $mock->lastCall()['method']);
        $this->assertStringContainsString('/api/v1/messages/m1', $mock->lastCall()['path']);
    }

    public function testConversationDeleteMessageSuccess(): void
    {
        $mock = new MockHttpHelper('{"status":"deleted","message_id":"m1"}');
        $svc = new ConversationService($mock);
        $result = $svc->deleteMessage('c1', 'm1');
        $this->assertSame('deleted', $result['status']);
        $this->assertStringContainsString('/api/v1/messages/m1', $mock->lastCall()['path']);
    }

    // ── OODA-39: Workspace Service - Stats Method ─────────────────

    public function testWorkspaceStats(): void
    {
        $mock = new MockHttpHelper('{"document_count":50,"entity_count":200,"relationship_count":150}');
        $svc = new WorkspaceService($mock);
        $result = $svc->stats('ws1');
        $this->assertSame(50, $result['document_count']);
        $this->assertSame(200, $result['entity_count']);
        $this->assertStringContainsString('/api/v1/workspaces/ws1/stats', $mock->lastCall()['path']);
    }

    public function testWorkspaceStatsEmpty(): void
    {
        $mock = new MockHttpHelper('{"document_count":0,"entity_count":0,"relationship_count":0}');
        $svc = new WorkspaceService($mock);
        $result = $svc->stats('ws1');
        $this->assertSame(0, $result['document_count']);
    }

    // ── OODA-39: URL Encoding Tests ───────────────────────────────

    public function testChatCompletionsWithConversationNullSafe(): void
    {
        $mock = new MockHttpHelper('{"message":"Response","conversation_id":"c1"}');
        $svc = new ChatService($mock);
        $result = $svc->completionsWithConversation('c1', '', 'hybrid');
        $this->assertSame('', $mock->lastCall()['body']['message']);
    }

    public function testWorkspaceStatsWithDifferentIds(): void
    {
        $mock = new MockHttpHelper('{"document_count":10}');
        $svc = new WorkspaceService($mock);
        $svc->stats('ws-uuid-1234');
        $this->assertStringContainsString('ws-uuid-1234', $mock->lastCall()['path']);
    }

    public function testConversationMessageOperationsSequence(): void
    {
        $mock = new MockHttpHelper('{"id":"m1","content":"test"}');
        $svc = new ConversationService($mock);
        // Create then update
        $svc->createMessage('c1', 'initial', 'user');
        $this->assertSame('POST', $mock->lastCall()['method']);
        $svc->updateMessage('c1', 'm1', 'updated');
        $this->assertSame('PATCH', $mock->lastCall()['method']);
    }

    // ── OODA-39: Error Handling Tests ─────────────────────────────

    public function testWorkspaceStatsUnauthorized(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{"error":"Unauthorized"}', 401);
        $svc = new WorkspaceService($mock);
        $this->expectException(ApiError::class);
        $svc->stats('ws1');
    }

    public function testConversationUpdateMessageNotFound(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{"error":"Message not found"}', 404);
        $svc = new ConversationService($mock);
        $this->expectException(ApiError::class);
        $svc->updateMessage('c1', 'missing', 'text');
    }

    public function testConversationDeleteMessageNotFound(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{"error":"Not found"}', 404);
        $svc = new ConversationService($mock);
        $this->expectException(ApiError::class);
        $svc->deleteMessage('c1', 'missing');
    }

    // ── OODA-48: Additional Edge Case Tests ─────────────────────────────

    public function testDocumentsListEmptyOODA48(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{"documents":[],"pagination":{}}');
        $svc = new DocumentService($mock);
        $result = $svc->list();
        $this->assertEquals('GET', $mock->lastCall()['method']);
        $this->assertArrayHasKey('documents', $result);
    }

    public function testEntitiesListEmptyOODA48(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{"items":[],"total":0}');
        $svc = new EntityService($mock);
        $result = $svc->list();
        $this->assertStringContainsString('/entities', $mock->lastCall()['path']);
        $this->assertSame(0, $result['total']);
    }

    public function testEntitiesCreateSuccessOODA48(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{"id":"e1","name":"Test","type":"PERSON"}');
        $svc = new EntityService($mock);
        $result = $svc->create('Test', 'PERSON', 'desc', 'doc-1');
        $this->assertEquals('POST', $mock->lastCall()['method']);
        $this->assertSame('e1', $result['id']);
    }

    public function testPipelineStatusIdleOODA48(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{"is_busy":false,"pending_tasks":0}');
        $svc = new PipelineService($mock);
        $result = $svc->status();
        $this->assertStringContainsString('/status', $mock->lastCall()['path']);
        $this->assertFalse($result['is_busy']);
    }

    public function testPipelineStatusBusyOODA48(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{"is_busy":true,"pending_tasks":10}');
        $svc = new PipelineService($mock);
        $result = $svc->status();
        $this->assertTrue($result['is_busy']);
        $this->assertSame(10, $result['pending_tasks']);
    }

    public function testTasksGetCompletedOODA48(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{"track_id":"t1","status":"completed"}');
        $svc = new TaskService($mock);
        $result = $svc->get('t1');
        $this->assertStringContainsString('/tasks/t1', $mock->lastCall()['path']);
        $this->assertSame('completed', $result['status']);
    }

    public function testTasksCancelSuccessOODA48(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{"success":true}');
        $svc = new TaskService($mock);
        $result = $svc->cancel('t1');
        $this->assertStringContainsString('/cancel', $mock->lastCall()['path']);
        $this->assertTrue($result['success']);
    }

    public function testModelsCatalogEmptyOODA48(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{"providers":[]}');
        $svc = new ModelService($mock);
        $result = $svc->catalog();
        $this->assertStringContainsString('/models', $mock->lastCall()['path']);
        $this->assertEmpty($result['providers']);
    }

    public function testModelsHealthOODA48(): void
    {
        $mock = (new MockHttpHelper())->willReturn('[{"provider":"ollama","healthy":true}]');
        $svc = new ModelService($mock);
        $result = $svc->health();
        $this->assertStringContainsString('/health', $mock->lastCall()['path']);
        $this->assertIsArray($result);
    }

    public function testCostsSummaryOODA48(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{"total_cost_usd":100.50,"document_count":50}');
        $svc = new CostService($mock);
        $result = $svc->summary();
        $this->assertStringContainsString('/summary', $mock->lastCall()['path']);
        $this->assertSame(50, $result['document_count']);
    }

    public function testFoldersListEmptyOODA48(): void
    {
        $mock = (new MockHttpHelper())->willReturn('[]');
        $svc = new FolderService($mock);
        $result = $svc->list();
        $this->assertEquals('GET', $mock->lastCall()['method']);
        $this->assertEmpty($result);
    }

    public function testFoldersCreateSuccessOODA48(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{"id":"f1","name":"TestFolder"}');
        $svc = new FolderService($mock);
        $result = $svc->create('TestFolder');
        $this->assertEquals('POST', $mock->lastCall()['method']);
        $this->assertSame('f1', $result['id']);
    }

    public function testConversationsGetOODA48(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{"id":"c1","title":"Test Conv"}');
        $svc = new ConversationService($mock);
        $result = $svc->get('c1');
        $this->assertStringContainsString('/conversations/c1', $mock->lastCall()['path']);
        $this->assertSame('Test Conv', $result['title']);
    }

    public function testRelationshipsListEmptyOODA48(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{"items":[],"total":0}');
        $svc = new RelationshipService($mock);
        $result = $svc->list();
        $this->assertStringContainsString('/relationships', $mock->lastCall()['path']);
        $this->assertSame(0, $result['total']);
    }

    public function testUsersListEmptyOODA48(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{"users":[]}');
        $svc = new UserService($mock);
        $result = $svc->list();
        $this->assertStringContainsString('/users', $mock->lastCall()['path']);
        $this->assertEmpty($result['users']);
    }

    public function testTenantsGetOODA48(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{"id":"t1","name":"Test Tenant"}');
        $svc = new TenantService($mock);
        $result = $svc->get('t1');
        $this->assertStringContainsString('/tenants/t1', $mock->lastCall()['path']);
        $this->assertSame('Test Tenant', $result['name']);
    }

    public function testApiKeysListEmptyOODA48(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{"keys":[]}');
        $svc = new ApiKeyService($mock);
        $result = $svc->list();
        $this->assertStringContainsString('/api-keys', $mock->lastCall()['path']);
        $this->assertEmpty($result['keys']);
    }

    public function testApiKeysRevokeOODA48(): void
    {
        $mock = (new MockHttpHelper())->willReturn('{"revoked":true}');
        $svc = new ApiKeyService($mock);
        $svc->revoke('key-1');
        $this->assertEquals('DELETE', $mock->lastCall()['method']);
        $this->assertStringContainsString('/api/v1/api-keys/key-1', $mock->lastCall()['path']);
    }
}

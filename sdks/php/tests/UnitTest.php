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
}

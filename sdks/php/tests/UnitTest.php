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
}

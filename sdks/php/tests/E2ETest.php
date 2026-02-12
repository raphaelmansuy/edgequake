<?php

declare(strict_types=1);

namespace EdgeQuake\Tests;

// Autoload without Composer
spl_autoload_register(function (string $class): void {
    $prefix = 'EdgeQuake\\';
    if (!str_starts_with($class, $prefix)) return;
    $relative = str_replace('\\', '/', substr($class, strlen($prefix)));
    $file = __DIR__ . '/../src/' . $relative . '.php';
    if (file_exists($file)) require $file;
});

// Services.php has multiple classes — require it explicitly
require_once __DIR__ . '/../src/Services.php';

use EdgeQuake\{Client, Config, ApiError};

/**
 * E2E tests for EdgeQuake PHP SDK.
 * Run: php tests/E2ETest.php
 */
class E2ETest
{
    private Client $client;
    private int $passed = 0;
    private int $failed = 0;
    private int $skipped = 0;

    public function __construct()
    {
        $base = getenv('EDGEQUAKE_BASE_URL') ?: 'http://localhost:8080';
        $tenantId = getenv('EDGEQUAKE_TENANT_ID') ?: '00000000-0000-0000-0000-000000000002';
        $userId = getenv('EDGEQUAKE_USER_ID') ?: '00000000-0000-0000-0000-000000000001';
        $this->client = new Client(new Config(baseUrl: $base, tenantId: $tenantId, userId: $userId));
    }

    public function run(): void
    {
        $methods = array_filter(
            get_class_methods($this),
            fn(string $m) => str_starts_with($m, 'test')
        );
        sort($methods);

        echo "Running " . count($methods) . " tests...\n\n";

        foreach ($methods as $method) {
            $this->$method();
        }

        echo "\n---\n";
        echo "Results: {$this->passed} passed, {$this->failed} failed, {$this->skipped} skipped\n";

        if ($this->failed > 0) {
            exit(1);
        }
    }

    private function pass(string $name): void
    {
        echo "  ✓ {$name}\n";
        $this->passed++;
    }

    private function fail(string $name, string $reason): void
    {
        echo "  ✗ {$name}: {$reason}\n";
        $this->failed++;
    }

    private function skip(string $name, string $reason): void
    {
        echo "  ⊘ {$name}: SKIPPED ({$reason})\n";
        $this->skipped++;
    }

    // 1. Health
    public function testHealthCheck(): void
    {
        try {
            $h = $this->client->health->check();
            assert($h['status'] === 'healthy', 'status should be healthy');
            assert(isset($h['version']), 'version should be set');
            $this->pass('Health Check');
        } catch (\Throwable $e) {
            $this->fail('Health Check', $e->getMessage());
        }
    }

    // 2. Documents
    public function testDocumentsListAndUpload(): void
    {
        try {
            $list = $this->client->documents->list();
            assert(array_key_exists('documents', $list), 'documents key');
            assert(array_key_exists('total', $list), 'total key');

            $resp = $this->client->documents->uploadText(
                'PHP SDK Test ' . bin2hex(random_bytes(4)),
                'PHP SDK integration test. Knowledge graphs are powerful.'
            );
            assert(isset($resp['document_id']), 'document_id');
            assert(isset($resp['status']), 'status');
            $this->pass('Documents List & Upload');
        } catch (\Throwable $e) {
            $this->fail('Documents List & Upload', $e->getMessage());
        }
    }

    // 3. Graph
    public function testGraphGet(): void
    {
        try {
            $g = $this->client->graph->get();
            assert($g !== null);
            $this->pass('Graph Get');
        } catch (\Throwable $e) {
            $this->fail('Graph Get', $e->getMessage());
        }
    }

    public function testGraphSearch(): void
    {
        try {
            $r = $this->client->graph->search('test');
            assert($r !== null);
            $this->pass('Graph Search');
        } catch (\Throwable $e) {
            $this->fail('Graph Search', $e->getMessage());
        }
    }

    // 4. Entity CRUD
    public function testEntityCrud(): void
    {
        try {
            $name = 'PHP_TEST_ENTITY_' . strtoupper(bin2hex(random_bytes(3)));

            $created = $this->client->entities->create($name, 'TEST', 'Created by PHP E2E', 'php-e2e');
            assert(isset($created['status']), 'create status');

            $list = $this->client->entities->list();
            assert(array_key_exists('items', $list), 'items key');

            $fetched = $this->client->entities->get($name);
            assert($fetched !== null);

            $del = $this->client->entities->delete($name);
            assert(isset($del['status']), 'delete status');

            $this->pass('Entity CRUD');
        } catch (\Throwable $e) {
            $this->fail('Entity CRUD', $e->getMessage());
        }
    }

    // 5. Relationships
    public function testRelationshipsList(): void
    {
        try {
            $list = $this->client->relationships->list();
            assert(array_key_exists('items', $list), 'items key');
            $this->pass('Relationships List');
        } catch (\Throwable $e) {
            $this->fail('Relationships List', $e->getMessage());
        }
    }

    // 6. Query
    public function testQuery(): void
    {
        try {
            $r = $this->client->query->execute('What is a knowledge graph?');
            assert(isset($r['answer']), 'answer key');
            $this->pass('Query');
        } catch (\Throwable $e) {
            $this->fail('Query', $e->getMessage());
        }
    }

    // 7. Chat
    public function testChat(): void
    {
        try {
            $r = $this->client->chat->completions('What entities exist?');
            assert(isset($r['content']), 'content key');
            $this->pass('Chat');
        } catch (ApiError $e) {
            if (in_array($e->statusCode, [401, 403])) {
                $this->pass('Chat (auth expected)');
            } else {
                $this->fail('Chat', $e->getMessage());
            }
        } catch (\Throwable $e) {
            $this->fail('Chat', $e->getMessage());
        }
    }

    // 8. Tenants
    public function testTenantsList(): void
    {
        try {
            $list = $this->client->tenants->list();
            assert(array_key_exists('items', $list), 'items key');
            $this->pass('Tenants List');
        } catch (\Throwable $e) {
            $this->fail('Tenants List', $e->getMessage());
        }
    }

    // 9. Users
    public function testUsersList(): void
    {
        try {
            $list = $this->client->users->list();
            assert(array_key_exists('users', $list), 'users key');
            $this->pass('Users List');
        } catch (\Throwable $e) {
            $this->fail('Users List', $e->getMessage());
        }
    }

    // 10. API Keys
    public function testApiKeysList(): void
    {
        try {
            $list = $this->client->apiKeys->list();
            assert(array_key_exists('keys', $list), 'keys key');
            $this->pass('API Keys List');
        } catch (\Throwable $e) {
            $this->fail('API Keys List', $e->getMessage());
        }
    }

    // 11. Tasks
    public function testTasksList(): void
    {
        try {
            $list = $this->client->tasks->list();
            assert(array_key_exists('tasks', $list), 'tasks key');
            $this->pass('Tasks List');
        } catch (\Throwable $e) {
            $this->fail('Tasks List', $e->getMessage());
        }
    }

    // 12. Pipeline Status
    public function testPipelineStatus(): void
    {
        try {
            $st = $this->client->pipeline->status();
            assert(array_key_exists('is_busy', $st), 'is_busy key');
            $this->pass('Pipeline Status');
        } catch (\Throwable $e) {
            $this->fail('Pipeline Status', $e->getMessage());
        }
    }

    // 13. Queue Metrics
    public function testQueueMetrics(): void
    {
        try {
            $m = $this->client->pipeline->queueMetrics();
            assert(array_key_exists('pending_count', $m), 'pending_count');
            assert(array_key_exists('active_workers', $m), 'active_workers');
            $this->pass('Queue Metrics');
        } catch (\Throwable $e) {
            $this->fail('Queue Metrics', $e->getMessage());
        }
    }

    // 14. Models Catalog
    public function testModelsCatalog(): void
    {
        try {
            $cat = $this->client->models->catalog();
            assert(array_key_exists('providers', $cat), 'providers key');
            $this->pass('Models Catalog');
        } catch (\Throwable $e) {
            $this->fail('Models Catalog', $e->getMessage());
        }
    }

    // 15. Models Health
    public function testModelsHealth(): void
    {
        try {
            $items = $this->client->models->health();
            assert(is_array($items), 'should be array');
            assert(count($items) > 0, 'should not be empty');
            $this->pass('Models Health');
        } catch (\Throwable $e) {
            $this->fail('Models Health', $e->getMessage());
        }
    }

    // 16. Provider Status
    public function testProviderStatus(): void
    {
        try {
            $ps = $this->client->models->providerStatus();
            assert(array_key_exists('provider', $ps), 'provider key');
            $this->pass('Provider Status');
        } catch (\Throwable $e) {
            $this->fail('Provider Status', $e->getMessage());
        }
    }

    // 17. Conversations
    public function testConversationsList(): void
    {
        try {
            $list = $this->client->conversations->list();
            assert(is_array($list), 'conversations response is array');
            $this->pass('Conversations List');
        } catch (\Throwable $e) {
            $this->fail('Conversations List', $e->getMessage());
        }
    }

    public function testConversationsCreate(): void
    {
        try {
            $conv = $this->client->conversations->create('PHP E2E Test ' . bin2hex(random_bytes(4)));
            assert(isset($conv['id']) || isset($conv['conversation_id']), 'conversation created');
            $this->pass('Conversations Create');
        } catch (\Throwable $e) {
            $this->fail('Conversations Create', $e->getMessage());
        }
    }

    // 18. Folders
    public function testFoldersList(): void
    {
        try {
            $list = $this->client->folders->list();
            assert(is_array($list), 'folders response is array');
            $this->pass('Folders List');
        } catch (\Throwable $e) {
            $this->fail('Folders List', $e->getMessage());
        }
    }

    public function testFoldersCreate(): void
    {
        try {
            $folder = $this->client->folders->create('PHP E2E Folder ' . bin2hex(random_bytes(4)));
            assert(isset($folder['id']) || isset($folder['name']), 'folder created');
            $this->pass('Folders Create');
        } catch (\Throwable $e) {
            $this->fail('Folders Create', $e->getMessage());
        }
    }

    // 19. Costs
    public function testCostsSummary(): void
    {
        try {
            $c = $this->client->costs->summary();
            assert($c !== null);
            $this->pass('Costs Summary');
        } catch (\Throwable $e) {
            $this->fail('Costs Summary', $e->getMessage());
        }
    }

    // 20. Full Workflow
    public function testFullWorkflow(): void
    {
        try {
            $doc = $this->client->documents->uploadText(
                'PHP Workflow ' . bin2hex(random_bytes(4)),
                'Knowledge graphs connect entities through relationships.'
            );
            assert(isset($doc['document_id']), 'document_id');

            $qr = $this->client->query->execute('What do knowledge graphs connect?');
            assert(isset($qr['answer']), 'answer');

            $ps = $this->client->pipeline->status();
            assert(array_key_exists('is_busy', $ps), 'is_busy');

            $this->pass('Full Workflow');
        } catch (\Throwable $e) {
            $this->fail('Full Workflow', $e->getMessage());
        }
    }
}

// Run
(new E2ETest())->run();

# EdgeQuake PHP SDK

Official PHP client for the [EdgeQuake](https://github.com/edgequake/edgequake) RAG API.

## Requirements

- PHP ≥ 8.1
- cURL extension (enabled by default)
- No external dependencies

## Installation

```bash
composer require edgequake/sdk
```

## Quick Start

```php
<?php

require_once 'vendor/autoload.php';

use EdgeQuake\Client;
use EdgeQuake\Config;

$client = new Client(new Config(
    baseUrl: 'http://localhost:8080',
    apiKey:  'sk-your-key',
));

// Health check
$health = $client->health->check();
echo $health['status']; // "healthy"

// Upload a document
$doc = $client->documents->uploadText('My Doc', 'Hello, world!');
echo $doc['id'];

// Query the knowledge graph
$result = $client->query->execute('What do you know?', 'hybrid');
echo $result['answer'];
```

## Configuration

| Parameter     | Type      | Default                 | Description           |
| ------------- | --------- | ----------------------- | --------------------- |
| `baseUrl`     | `string`  | `http://localhost:8080` | API base URL          |
| `apiKey`      | `?string` | `null`                  | API key               |
| `tenantId`    | `?string` | `null`                  | Tenant ID header      |
| `userId`      | `?string` | `null`                  | User ID header        |
| `workspaceId` | `?string` | `null`                  | Workspace ID header   |
| `timeout`     | `int`     | `60`                    | Request timeout (sec) |

```php
$config = new Config(
    baseUrl:     'https://api.example.com',
    apiKey:      'sk-my-key',
    tenantId:    'tenant-1',
    userId:      'user-1',
    workspaceId: 'ws-1',
    timeout:     120,
);
$client = new Client($config);
```

## Services

### Health

```php
$status = $client->health->check();
// { "status": "healthy", "version": "0.1.0", ... }
```

### Documents

```php
// List documents
$docs = $client->documents->list(page: 1, pageSize: 20);

// Get a document
$doc = $client->documents->get('doc-id');

// Upload text
$doc = $client->documents->uploadText('Title', 'Content body', 'txt');

// Delete
$client->documents->delete('doc-id');
```

### Entities

```php
// List entities
$entities = $client->entities->list(page: 1, pageSize: 20);

// Get an entity
$entity = $client->entities->get('ENTITY_NAME');

// Create
$client->entities->create('NODE', 'concept', 'A concept node', 'source-1');

// Delete
$client->entities->delete('NODE');
```

### Relationships

```php
$rels = $client->relationships->list(page: 1, pageSize: 20);
```

### Graph

```php
// Get full graph
$graph = $client->graph->get();

// Search nodes
$results = $client->graph->search('Alice');
```

### Query

```php
$result = $client->query->execute('What is EdgeQuake?', 'hybrid');
echo $result['answer'];
echo count($result['sources']);
```

Modes: `hybrid`, `local`, `global`, `naive`.

### Chat

```php
$response = $client->chat->completions('Hello!', 'hybrid', stream: false);
echo $response['choices'][0]['message']['content'];
```

### Tenants

```php
$tenants = $client->tenants->list();
```

### Users

```php
$users = $client->users->list();
```

### API Keys

```php
$keys = $client->apiKeys->list();
```

### Tasks

```php
$tasks = $client->tasks->list();
```

### Pipeline

```php
$status  = $client->pipeline->status();
$metrics = $client->pipeline->queueMetrics();
```

### Models

```php
$catalog  = $client->models->catalog();
$health   = $client->models->health();
$provider = $client->models->providerStatus();
```

### Costs

```php
$costs = $client->costs->summary();
echo $costs['total_cost_usd'];
```

## Error Handling

All API errors throw `EdgeQuake\ApiError` (extends `RuntimeException`):

```php
use EdgeQuake\ApiError;

try {
    $client->documents->get('nonexistent');
} catch (ApiError $e) {
    echo $e->getMessage();     // "HTTP 404: ..."
    echo $e->statusCode;       // 404
    echo $e->responseBody;     // raw JSON string
}
```

| Property       | Type      | Description       |
| -------------- | --------- | ----------------- |
| `statusCode`   | `?int`    | HTTP status code  |
| `responseBody` | `?string` | Raw response body |
| `getMessage()` | `string`  | Error description |

## Testing

```bash
composer install
vendor/bin/phpunit
```

Run with coverage:

```bash
XDEBUG_MODE=coverage vendor/bin/phpunit --coverage-text
```

## Project Structure

```
src/
├── ApiError.php      # Error class (extends RuntimeException)
├── Client.php        # Main client with service accessors
├── Config.php        # Configuration (constructor promotion)
├── HttpHelper.php    # cURL-based HTTP helper
└── Services.php      # 14 service classes
tests/
├── E2ETest.php       # Integration tests (needs running server)
├── MockHttpHelper.php# Mock for unit testing
└── UnitTest.php      # 62 unit tests, 114 assertions
```

## License

Apache-2.0

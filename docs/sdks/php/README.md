---
title: "PHP SDK"
---

# PHP SDK

> **Product: v0.19.0** · SDK package maturity: **experimental** (~0.4.x track)

**Location:** `sdks/php`  
**Contract:** [`openapi.snapshot.json`](../../../edgequake_webui/openapi/openapi.snapshot.json)

## Status

Experimental client with unit tests and CI (`sdks/php/.github/workflows/test.yml`). Not yet published to Packagist as a first-class release — consume from the monorepo or generate from OpenAPI until publication is wired.

## Requirements

- PHP ≥ 8.1
- cURL extension

## Install (monorepo)

```bash
cd sdks/php
composer install
```

Point Composer at the path in your app, or copy `src/` until Packagist publication lands.

## Quick example

```php
<?php

require_once 'vendor/autoload.php';

use EdgeQuake\Client;
use EdgeQuake\Config;

$client = new Client(new Config(
    baseUrl: getenv('EDGEQUAKE_BASE_URL') ?: 'http://localhost:8080',
    apiKey:  getenv('EDGEQUAKE_API_KEY') ?: null,
    workspaceId: getenv('EDGEQUAKE_WORKSPACE_ID') ?: 'default',
));

$health = $client->health->check();
echo $health['status']; // "healthy"
```

## v0.19 API notes

- Set `workspaceId` (and tenant/user headers when auth is enabled) on `Config`.
- Task cancel: `POST /api/v1/tasks/{track_id}/cancel` — see [Ingestion cancel & fairness](../../ingestion-cancel-and-fairness.md). Typed helpers may lag Tier 1 SDKs; verify against OpenAPI.
- PDF vision uploads and progress SSE are not uniformly wrapped — use raw HTTP or OpenAPI for progress/cancel until parity lands.

## Test

```bash
cd sdks/php && composer install && vendor/bin/phpunit
```

## See also

- In-repo reference: `sdks/php/README.md`
- [Brutal assessment](../BRUTAL-ASSESSMENT.md)
- [Custom Clients](../../integrations/custom-clients.md)

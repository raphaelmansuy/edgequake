---
title: PHP SDK
description: Use the EdgeQuake PHP client (Composer package edgequake/sdk). Requires PHP 8.1+. Not on Packagist yet.
---

# PHP SDK

PHP client for EdgeQuake. Composer name: **`edgequake/sdk`**. Requires **PHP 8.1+**. **Not published** on Packagist; require from a path repository pointing at `sdks/php`.

```json
{
  "repositories": [{ "type": "path", "url": "sdks/php" }],
  "require": { "edgequake/sdk": "*" }
}
```

```php
<?php
use EdgeQuake\Client;

$client = new Client([
    'base_url' => 'http://localhost:8080',
    'api_key'  => 'eq-...',
]);

$docs = $client->documents->list(1, 20);
$answer = $client->query->execute('What is in my documents?', 'mix');
```

Class: `EdgeQuake\Client`. Connections are not wrapped. See [SDK overview](../README.md).

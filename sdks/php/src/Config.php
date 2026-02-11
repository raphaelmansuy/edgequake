<?php

declare(strict_types=1);

namespace EdgeQuake;

/**
 * Configuration for the EdgeQuake client.
 */
class Config
{
    public function __construct(
        public readonly string $baseUrl = 'http://localhost:8080',
        public readonly ?string $apiKey = null,
        public readonly ?string $tenantId = null,
        public readonly ?string $userId = null,
        public readonly ?string $workspaceId = null,
        public readonly int $timeout = 60,
    ) {}
}

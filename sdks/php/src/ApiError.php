<?php

declare(strict_types=1);

namespace EdgeQuake;

/**
 * HTTP error from the EdgeQuake API.
 */
class ApiError extends \RuntimeException
{
    public function __construct(
        string $message,
        public readonly ?int $statusCode = null,
        public readonly ?string $responseBody = null,
    ) {
        parent::__construct($message);
    }
}

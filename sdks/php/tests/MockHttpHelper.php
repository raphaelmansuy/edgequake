<?php

declare(strict_types=1);

namespace EdgeQuake\Tests;

use EdgeQuake\Config;
use EdgeQuake\HttpHelper;

/**
 * Mock HTTP helper that returns predefined responses without making real HTTP calls.
 * WHY: Enables stateless unit testing of all service methods.
 * OODA-33: Added put() and patch() methods.
 */
class MockHttpHelper extends HttpHelper
{
    /** @var list<array{method: string, path: string, body: ?array}> */
    public array $calls = [];
    private string $nextResponse;
    private int $nextStatus;

    public function __construct(string $response = '{}', int $status = 200)
    {
        parent::__construct(new Config());
        $this->nextResponse = $response;
        $this->nextStatus = $status;
    }

    public function willReturn(string $json, int $status = 200): self
    {
        $this->nextResponse = $json;
        $this->nextStatus = $status;
        return $this;
    }

    public function put(string $path, ?array $body = null): array
    {
        return $this->request('PUT', $path, $body);
    }

    public function patch(string $path, ?array $body = null): array
    {
        return $this->request('PATCH', $path, $body);
    }

    protected function request(string $method, string $path, ?array $body = null): array
    {
        $this->calls[] = ['method' => $method, 'path' => $path, 'body' => $body];

        if ($this->nextStatus < 200 || $this->nextStatus >= 300) {
            throw new \EdgeQuake\ApiError(
                "HTTP {$this->nextStatus}: {$this->nextResponse}",
                statusCode: $this->nextStatus,
                responseBody: $this->nextResponse,
            );
        }

        $decoded = json_decode($this->nextResponse, true);
        return $decoded ?? [];
    }

    protected function requestRaw(string $method, string $path, ?array $body = null): string
    {
        $this->calls[] = ['method' => $method, 'path' => $path, 'body' => $body];

        if ($this->nextStatus < 200 || $this->nextStatus >= 300) {
            throw new \EdgeQuake\ApiError(
                "HTTP {$this->nextStatus}: {$this->nextResponse}",
                statusCode: $this->nextStatus,
                responseBody: $this->nextResponse,
            );
        }

        return $this->nextResponse;
    }

    public function lastCall(): ?array
    {
        return end($this->calls) ?: null;
    }
}

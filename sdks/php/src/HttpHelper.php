<?php

declare(strict_types=1);

namespace EdgeQuake;

/**
 * Internal HTTP helper using cURL.
 * WHY: Zero external dependencies — cURL is built-in.
 */
class HttpHelper
{
    public function __construct(private readonly Config $config) {}

    public function get(string $path): array
    {
        return $this->request('GET', $path);
    }

    public function post(string $path, ?array $body = null): array
    {
        return $this->request('POST', $path, $body);
    }

    public function put(string $path, ?array $body = null): array
    {
        return $this->request('PUT', $path, $body);
    }

    public function patch(string $path, ?array $body = null): array
    {
        return $this->request('PATCH', $path, $body);
    }

    public function delete(string $path): array
    {
        return $this->request('DELETE', $path);
    }

    public function getRaw(string $path): string
    {
        return $this->requestRaw('GET', $path);
    }

    protected function request(string $method, string $path, ?array $body = null): array
    {
        $raw = $this->requestRaw($method, $path, $body);
        $decoded = json_decode($raw, true);
        if ($decoded === null && $raw !== 'null') {
            throw new ApiError("JSON decode failed: " . json_last_error_msg());
        }
        return $decoded ?? [];
    }

    protected function requestRaw(string $method, string $path, ?array $body = null): string
    {
        $url = rtrim($this->config->baseUrl, '/') . $path;
        $ch = curl_init($url);

        $headers = [
            'Content-Type: application/json',
            'Accept: application/json',
        ];

        if ($this->config->apiKey !== null) {
            $headers[] = 'X-API-Key: ' . $this->config->apiKey;
        }
        if ($this->config->tenantId !== null) {
            $headers[] = 'X-Tenant-ID: ' . $this->config->tenantId;
        }
        if ($this->config->userId !== null) {
            $headers[] = 'X-User-ID: ' . $this->config->userId;
        }
        if ($this->config->workspaceId !== null) {
            $headers[] = 'X-Workspace-ID: ' . $this->config->workspaceId;
        }

        curl_setopt_array($ch, [
            CURLOPT_CUSTOMREQUEST => $method,
            CURLOPT_HTTPHEADER => $headers,
            CURLOPT_RETURNTRANSFER => true,
            CURLOPT_TIMEOUT => $this->config->timeout,
            CURLOPT_CONNECTTIMEOUT => $this->config->timeout,
        ]);

        if ($body !== null) {
            curl_setopt($ch, CURLOPT_POSTFIELDS, json_encode($body));
        } elseif (in_array($method, ['POST', 'PUT', 'PATCH'])) {
            curl_setopt($ch, CURLOPT_POSTFIELDS, '{}');
        }

        $response = curl_exec($ch);
        $statusCode = (int)curl_getinfo($ch, CURLINFO_HTTP_CODE);
        $error = curl_error($ch);

        if ($response === false) {
            throw new ApiError("cURL error: {$error}");
        }

        if ($statusCode < 200 || $statusCode >= 300) {
            throw new ApiError(
                "HTTP {$statusCode}: {$response}",
                statusCode: $statusCode,
                responseBody: $response,
            );
        }

        return $response;
    }

    // OODA-39: File upload support.

    /**
     * Upload a file via multipart/form-data.
     *
     * @param string $path API endpoint
     * @param string $filePath Local file path
     * @param string $fieldName Form field name (default: 'file')
     * @param array $extraFields Additional form fields
     * @return array Decoded JSON response
     */
    public function upload(string $path, string $filePath, string $fieldName = 'file', array $extraFields = []): array
    {
        if (!file_exists($filePath)) {
            throw new ApiError("File not found: {$filePath}");
        }

        $url = rtrim($this->config->baseUrl, '/') . $path;
        $ch = curl_init($url);

        $headers = ['Accept: application/json'];
        if ($this->config->apiKey !== null) {
            $headers[] = 'X-API-Key: ' . $this->config->apiKey;
        }
        if ($this->config->tenantId !== null) {
            $headers[] = 'X-Tenant-ID: ' . $this->config->tenantId;
        }
        if ($this->config->workspaceId !== null) {
            $headers[] = 'X-Workspace-ID: ' . $this->config->workspaceId;
        }

        $postData = array_merge([
            $fieldName => new \CURLFile($filePath),
        ], $extraFields);

        curl_setopt_array($ch, [
            CURLOPT_POST => true,
            CURLOPT_HTTPHEADER => $headers,
            CURLOPT_POSTFIELDS => $postData,
            CURLOPT_RETURNTRANSFER => true,
            CURLOPT_TIMEOUT => $this->config->timeout,
        ]);

        $response = curl_exec($ch);
        $statusCode = (int)curl_getinfo($ch, CURLINFO_HTTP_CODE);
        curl_close($ch);

        if ($response === false) {
            throw new ApiError("cURL error during upload");
        }

        if ($statusCode < 200 || $statusCode >= 300) {
            throw new ApiError("HTTP {$statusCode}: {$response}", statusCode: $statusCode, responseBody: $response);
        }

        return json_decode($response, true) ?? [];
    }

    // OODA-39: Streaming POST support.

    /**
     * Execute streaming POST and yield chunks.
     *
     * @param string $path API endpoint
     * @param array $body Request body
     * @return \Generator Yields string chunks as Server-Sent Events
     */
    public function streamPost(string $path, array $body): \Generator
    {
        $url = rtrim($this->config->baseUrl, '/') . $path;
        $ch = curl_init($url);

        $headers = [
            'Content-Type: application/json',
            'Accept: text/event-stream',
        ];

        if ($this->config->apiKey !== null) {
            $headers[] = 'X-API-Key: ' . $this->config->apiKey;
        }
        if ($this->config->workspaceId !== null) {
            $headers[] = 'X-Workspace-ID: ' . $this->config->workspaceId;
        }

        $buffer = '';

        curl_setopt_array($ch, [
            CURLOPT_POST => true,
            CURLOPT_HTTPHEADER => $headers,
            CURLOPT_POSTFIELDS => json_encode($body),
            CURLOPT_RETURNTRANSFER => false,
            CURLOPT_TIMEOUT => 0, // No timeout for streaming
            CURLOPT_WRITEFUNCTION => function ($ch, $data) use (&$buffer) {
                $buffer .= $data;
                return strlen($data);
            },
        ]);

        curl_exec($ch);
        $statusCode = (int)curl_getinfo($ch, CURLINFO_HTTP_CODE);
        curl_close($ch);

        if ($statusCode < 200 || $statusCode >= 300) {
            throw new ApiError("HTTP {$statusCode}: {$buffer}", statusCode: $statusCode, responseBody: $buffer);
        }

        // Parse SSE chunks
        $lines = explode("\n", $buffer);
        foreach ($lines as $line) {
            if (str_starts_with($line, 'data: ')) {
                $data = substr($line, 6);
                if ($data === '[DONE]') {
                    return;
                }
                yield $data;
            }
        }
    }
}

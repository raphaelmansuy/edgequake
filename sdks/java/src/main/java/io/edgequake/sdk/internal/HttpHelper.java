package io.edgequake.sdk.internal;

import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import io.edgequake.sdk.EdgeQuakeConfig;
import io.edgequake.sdk.EdgeQuakeException;

import java.io.IOException;
import java.io.ByteArrayOutputStream;
import java.net.URI;
import java.net.URLEncoder;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.nio.charset.StandardCharsets;
import java.time.Duration;
import java.util.List;
import java.util.Map;

/**
 * Internal HTTP helper wrapping java.net.http.HttpClient with EdgeQuake auth headers.
 *
 * WHY: Uses java.net.http.HttpClient (built into Java 11+) instead of OkHttp
 * to eliminate external HTTP dependencies. Jackson handles all JSON serialization.
 */
public class HttpHelper {

    private final HttpClient httpClient;
    private final ObjectMapper mapper;
    private final EdgeQuakeConfig config;

    public HttpHelper(EdgeQuakeConfig config) {
        this.config = config;
        this.mapper = new ObjectMapper()
                .configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false);
        this.httpClient = HttpClient.newBuilder()
                .connectTimeout(Duration.ofSeconds(config.timeoutSeconds()))
                .build();
    }

    public ObjectMapper mapper() { return mapper; }

    // ── GET ──────────────────────────────────────────────────────────────

    public <T> T get(String path, Map<String, String> params, Class<T> type) {
        var body = doRequest(buildGet(url(path, params)));
        return deserialize(body, type);
    }

    public <T> T get(String path, Map<String, String> params, TypeReference<T> type) {
        var body = doRequest(buildGet(url(path, params)));
        return deserialize(body, type);
    }

    // OODA-40: Add getRaw for raw string responses (e.g., metrics).
    public String getRaw(String path, Map<String, String> params) {
        return doRequest(buildGet(url(path, params)));
    }

    // ── POST ─────────────────────────────────────────────────────────────

    public <T> T post(String path, Object payload, Class<T> type) {
        var body = doRequest(buildPost(url(path, null), serialize(payload)));
        return deserialize(body, type);
    }

    public void postNoContent(String path, Object payload) {
        doRequest(buildPost(url(path, null), serialize(payload)));
    }

    // OODA-40: Add postRaw for raw string responses (e.g., streaming).
    public String postRaw(String path, Object payload) {
        return doRequest(buildPost(url(path, null), serialize(payload)));
    }

    // ── DELETE ───────────────────────────────────────────────────────────

    /** DELETE that may return a body or empty. Does NOT throw on 204/200. */
    public String delete(String path) {
        return doRequest(buildDelete(url(path, null)));
    }

    public <T> T delete(String path, Class<T> type) {
        var body = doRequest(buildDelete(url(path, null)));
        if (body == null || body.isBlank()) return null;
        return deserialize(body, type);
    }

    // ── PATCH ────────────────────────────────────────────────────────────

    public void patch(String path, Object payload) {
        doRequest(buildPatch(url(path, null), serialize(payload)));
    }

    // OODA-40: Added patch with response type.
    public <T> T patch(String path, Object payload, Class<T> type) {
        var body = doRequest(buildPatch(url(path, null), serialize(payload)));
        return deserialize(body, type);
    }

    // ── PUT ──────────────────────────────────────────────────────────────

    // OODA-40: Added PUT method.
    public <T> T put(String path, Object payload, Class<T> type) {
        var body = doRequest(buildPut(url(path, null), serialize(payload)));
        return deserialize(body, type);
    }

    public <T> T uploadMany(String path, List<String> filePaths, String fieldName, Map<String, String> extraFields, Class<T> type) {
        String boundary = "----edgequake-sdk-" + System.currentTimeMillis();
        byte[] body = buildMultipartBody(boundary, filePaths, fieldName, extraFields);
        var req = baseRequest(url(path, null))
                .header("Content-Type", "multipart/form-data; boundary=" + boundary)
                .POST(HttpRequest.BodyPublishers.ofByteArray(body))
                .build();
        return deserialize(doRequest(req), type);
    }

    // ── Internal ─────────────────────────────────────────────────────────

    private String url(String path, Map<String, String> params) {
        var sb = new StringBuilder(config.baseUrl());
        sb.append(path);
        if (params != null && !params.isEmpty()) {
            sb.append('?');
            var first = true;
            for (var entry : params.entrySet()) {
                if (!first) sb.append('&');
                sb.append(URLEncoder.encode(entry.getKey(), StandardCharsets.UTF_8));
                sb.append('=');
                sb.append(URLEncoder.encode(entry.getValue(), StandardCharsets.UTF_8));
                first = false;
            }
        }
        return sb.toString();
    }

    private HttpRequest.Builder baseRequest(String url) {
        var builder = HttpRequest.newBuilder()
                .uri(URI.create(url))
                .timeout(Duration.ofSeconds(config.timeoutSeconds()));

        // WHY: EdgeQuake API uses X-API-Key header (not Authorization: Bearer).
        if (config.apiKey() != null) {
            builder.header("X-API-Key", config.apiKey());
        }
        if (config.tenantId() != null) {
            builder.header("X-Tenant-ID", config.tenantId());
        }
        if (config.userId() != null) {
            builder.header("X-User-ID", config.userId());
        }
        if (config.workspaceId() != null) {
            builder.header("X-Workspace-ID", config.workspaceId());
        }
        return builder;
    }

    private HttpRequest buildGet(String url) {
        return baseRequest(url).GET().build();
    }

    private HttpRequest buildPost(String url, String json) {
        return baseRequest(url)
                .header("Content-Type", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(json != null ? json : "{}"))
                .build();
    }

    private HttpRequest buildDelete(String url) {
        return baseRequest(url).DELETE().build();
    }

    private HttpRequest buildPatch(String url, String json) {
        return baseRequest(url)
                .header("Content-Type", "application/json")
                .method("PATCH", HttpRequest.BodyPublishers.ofString(json != null ? json : "{}"))
                .build();
    }

    // OODA-40: Added buildPut method.
    private HttpRequest buildPut(String url, String json) {
        return baseRequest(url)
                .header("Content-Type", "application/json")
                .PUT(HttpRequest.BodyPublishers.ofString(json != null ? json : "{}"))
                .build();
    }

    private String doRequest(HttpRequest request) {
        try {
            var response = httpClient.send(request, HttpResponse.BodyHandlers.ofString());
            var status = response.statusCode();
            if (status >= 200 && status < 300) {
                return response.body();
            }
            throw new EdgeQuakeException(status, response.body());
        } catch (EdgeQuakeException e) {
            throw e;
        } catch (IOException | InterruptedException e) {
            if (e instanceof InterruptedException) Thread.currentThread().interrupt();
            throw new EdgeQuakeException("HTTP request failed: " + e.getMessage(), e);
        }
    }

    private byte[] buildMultipartBody(String boundary, List<String> filePaths, String fieldName, Map<String, String> extraFields) {
        try {
            var out = new ByteArrayOutputStream();
            for (String filePath : filePaths) {
                var filename = Paths.get(filePath).getFileName().toString();
                out.write(("--" + boundary + "\r\n").getBytes(StandardCharsets.UTF_8));
                out.write(("Content-Disposition: form-data; name=\"" + fieldName + "\"; filename=\"" + filename + "\"\r\n").getBytes(StandardCharsets.UTF_8));
                out.write("Content-Type: application/octet-stream\r\n\r\n".getBytes(StandardCharsets.UTF_8));
                out.write(Files.readAllBytes(Paths.get(filePath)));
                out.write("\r\n".getBytes(StandardCharsets.UTF_8));
            }
            if (extraFields != null) {
                for (var e : extraFields.entrySet()) {
                    out.write(("--" + boundary + "\r\n").getBytes(StandardCharsets.UTF_8));
                    out.write(("Content-Disposition: form-data; name=\"" + e.getKey() + "\"\r\n\r\n").getBytes(StandardCharsets.UTF_8));
                    out.write(e.getValue().getBytes(StandardCharsets.UTF_8));
                    out.write("\r\n".getBytes(StandardCharsets.UTF_8));
                }
            }
            out.write(("--" + boundary + "--\r\n").getBytes(StandardCharsets.UTF_8));
            return out.toByteArray();
        } catch (IOException e) {
            throw new EdgeQuakeException("Multipart build failed: " + e.getMessage(), e);
        }
    }

    private String serialize(Object payload) {
        if (payload == null) return null;
        try {
            return mapper.writeValueAsString(payload);
        } catch (Exception e) {
            throw new EdgeQuakeException("JSON serialization failed: " + e.getMessage(), e);
        }
    }

    private <T> T deserialize(String body, Class<T> type) {
        if (body == null || body.isBlank()) return null;
        try {
            return mapper.readValue(body, type);
        } catch (Exception e) {
            throw new EdgeQuakeException("JSON deserialization failed: " + e.getMessage(), e);
        }
    }

    private <T> T deserialize(String body, TypeReference<T> type) {
        if (body == null || body.isBlank()) return null;
        try {
            return mapper.readValue(body, type);
        } catch (Exception e) {
            throw new EdgeQuakeException("JSON deserialization failed: " + e.getMessage(), e);
        }
    }
}

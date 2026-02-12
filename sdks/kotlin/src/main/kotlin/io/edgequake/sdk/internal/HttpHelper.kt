package io.edgequake.sdk.internal

import com.fasterxml.jackson.annotation.JsonInclude
import com.fasterxml.jackson.core.type.TypeReference
import com.fasterxml.jackson.databind.DeserializationFeature
import com.fasterxml.jackson.module.kotlin.jacksonObjectMapper
import io.edgequake.sdk.EdgeQuakeConfig
import io.edgequake.sdk.EdgeQuakeException
import java.net.URI
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse
import java.time.Duration

/**
 * Internal HTTP helper using java.net.http.HttpClient.
 * WHY: Zero external HTTP dependencies — JDK 11+ built-in.
 */
open class HttpHelper(@PublishedApi internal val config: EdgeQuakeConfig) {

    @PublishedApi internal val mapper = jacksonObjectMapper().apply {
        configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false)
        setSerializationInclusion(JsonInclude.Include.NON_NULL)
    }

    @PublishedApi internal val httpClient: HttpClient = HttpClient.newBuilder()
        .connectTimeout(Duration.ofSeconds(config.timeoutSeconds))
        .build()

    // ── Public API ───────────────────────────────────────────────────

    inline fun <reified T> get(path: String): T =
        execute(buildRequest(path, "GET", null))

    inline fun <reified T> post(path: String, body: Any? = null): T =
        execute(buildRequest(path, "POST", body))

    inline fun <reified T> put(path: String, body: Any? = null): T =
        execute(buildRequest(path, "PUT", body))

    inline fun <reified T> patch(path: String, body: Any? = null): T =
        execute(buildRequest(path, "PATCH", body))

    inline fun <reified T> delete(path: String): T =
        execute(buildRequest(path, "DELETE", null))

    open fun deleteRaw(path: String): String {
        val req = buildRequest(path, "DELETE", null)
        val resp = httpClient.send(req, HttpResponse.BodyHandlers.ofString())
        if (resp.statusCode() !in 200..299) {
            throw EdgeQuakeException(
                "HTTP ${resp.statusCode()}: ${resp.body()}",
                resp.statusCode(), resp.body()
            )
        }
        return resp.body()
    }

    open fun getRaw(path: String): String {
        val req = buildRequest(path, "GET", null)
        val resp = httpClient.send(req, HttpResponse.BodyHandlers.ofString())
        if (resp.statusCode() !in 200..299) {
            throw EdgeQuakeException(
                "HTTP ${resp.statusCode()}: ${resp.body()}",
                resp.statusCode(), resp.body()
            )
        }
        return resp.body()
    }

    open fun postRaw(path: String, body: Any? = null): String {
        val req = buildRequest(path, "POST", body)
        val resp = httpClient.send(req, HttpResponse.BodyHandlers.ofString())
        if (resp.statusCode() !in 200..299) {
            throw EdgeQuakeException(
                "HTTP ${resp.statusCode()}: ${resp.body()}",
                resp.statusCode(), resp.body()
            )
        }
        return resp.body()
    }

    // ── Internals ────────────────────────────────────────────────────

    fun buildRequest(path: String, method: String, body: Any?): HttpRequest {
        val url = "${config.baseUrl}$path"
        val builder = HttpRequest.newBuilder()
            .uri(URI.create(url))
            .timeout(Duration.ofSeconds(config.timeoutSeconds))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")

        config.apiKey?.let { builder.header("X-API-Key", it) }
        config.tenantId?.let { builder.header("X-Tenant-ID", it) }
        config.userId?.let { builder.header("X-User-ID", it) }
        config.workspaceId?.let { builder.header("X-Workspace-ID", it) }

        val bodyPublisher = if (body != null) {
            HttpRequest.BodyPublishers.ofString(mapper.writeValueAsString(body))
        } else if (method == "POST" || method == "PUT" || method == "PATCH") {
            HttpRequest.BodyPublishers.ofString("{}")
        } else {
            HttpRequest.BodyPublishers.noBody()
        }

        builder.method(method, bodyPublisher)
        return builder.build()
    }

    inline fun <reified T> execute(request: HttpRequest): T {
        try {
            val resp = httpClient.send(request, HttpResponse.BodyHandlers.ofString())
            if (resp.statusCode() !in 200..299) {
                throw EdgeQuakeException(
                    "HTTP ${resp.statusCode()}: ${resp.body()}",
                    resp.statusCode(), resp.body()
                )
            }
            // WHY: 204 No Content returns empty body — handle gracefully
            val body = resp.body()
            if (body.isNullOrBlank()) {
                @Suppress("UNCHECKED_CAST")
                return when {
                    T::class == Unit::class -> Unit as T
                    T::class == Map::class -> emptyMap<String, Any?>() as T
                    T::class == String::class -> "" as T
                    else -> throw EdgeQuakeException("Empty response body for type ${T::class}")
                }
            }
            return mapper.readValue(body, object : TypeReference<T>() {})
        } catch (e: EdgeQuakeException) {
            throw e
        } catch (e: Exception) {
            throw EdgeQuakeException("Request failed: ${e.message}", cause = e)
        }
    }
}

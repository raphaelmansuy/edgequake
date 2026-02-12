package io.edgequake.sdk

import io.edgequake.sdk.internal.HttpHelper
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse
import java.net.http.HttpResponse.BodyHandler
import java.nio.ByteBuffer
import java.util.concurrent.CompletableFuture
import java.util.concurrent.Flow
import javax.net.ssl.SSLContext
import javax.net.ssl.SSLParameters
import java.net.Authenticator
import java.net.CookieHandler
import java.net.ProxySelector
import java.time.Duration
import java.util.Optional

/**
 * Fake HttpClient for unit testing.
 *
 * Captures requests and returns pre-configured responses
 * without network I/O.
 */
class FakeHttpClient : HttpClient() {

    data class CapturedRequest(
        val method: String,
        val uri: String,
        val body: String?
    )

    val captured = mutableListOf<CapturedRequest>()
    var responseBody: String = "{}"
    var responseCode: Int = 200
    var shouldThrow: Exception? = null

    fun respondWith(json: String, code: Int = 200) {
        responseBody = json
        responseCode = code
        shouldThrow = null
    }

    fun respondWithError(code: Int, body: String = """{"error":"mock error"}""") {
        responseBody = body
        responseCode = code
        shouldThrow = null
    }

    fun throwOnSend(ex: Exception) {
        shouldThrow = ex
    }

    fun lastRequest(): CapturedRequest = captured.last()
    fun clear() { captured.clear() }

    @Suppress("UNCHECKED_CAST")
    override fun <T> send(
        request: HttpRequest,
        responseBodyHandler: BodyHandler<T>
    ): HttpResponse<T> {
        // Capture request body
        val bodyStr = request.bodyPublisher().map { pub ->
            val sb = StringBuilder()
            pub.subscribe(object : Flow.Subscriber<ByteBuffer> {
                override fun onSubscribe(s: Flow.Subscription) { s.request(Long.MAX_VALUE) }
                override fun onNext(item: ByteBuffer) {
                    val arr = ByteArray(item.remaining())
                    item.get(arr)
                    sb.append(String(arr))
                }
                override fun onError(throwable: Throwable) {}
                override fun onComplete() {}
            })
            // Give subscriber a moment
            Thread.sleep(10)
            sb.toString().ifEmpty { null }
        }.orElse(null)

        captured.add(CapturedRequest(request.method(), request.uri().toString(), bodyStr))

        shouldThrow?.let { throw it }

        // Build a fake response
        return object : HttpResponse<T> {
            override fun statusCode() = responseCode
            override fun headers() = java.net.http.HttpHeaders.of(mapOf("content-type" to listOf("application/json"))) { _, _ -> true }
            override fun body() = responseBody as T
            override fun uri() = request.uri()
            override fun request() = request
            override fun previousResponse() = Optional.empty<HttpResponse<T>>()
            override fun sslSession() = Optional.empty<javax.net.ssl.SSLSession>()
            override fun version() = Version.HTTP_1_1
        }
    }

    override fun <T> sendAsync(
        request: HttpRequest,
        responseBodyHandler: BodyHandler<T>
    ) = CompletableFuture.completedFuture(send(request, responseBodyHandler))

    override fun <T : Any?> sendAsync(
        request: HttpRequest,
        responseBodyHandler: BodyHandler<T>,
        pushPromiseHandler: HttpResponse.PushPromiseHandler<T>
    ): CompletableFuture<HttpResponse<T>> = sendAsync(request, responseBodyHandler)

    // Required abstract method implementations
    override fun cookieHandler() = Optional.empty<CookieHandler>()
    override fun connectTimeout() = Optional.of(Duration.ofSeconds(30))
    override fun followRedirects() = Redirect.NEVER
    override fun proxy() = Optional.empty<ProxySelector>()
    override fun sslContext(): SSLContext = SSLContext.getDefault()
    override fun sslParameters() = SSLParameters()
    override fun authenticator() = Optional.empty<Authenticator>()
    override fun version() = Version.HTTP_1_1
    override fun executor() = Optional.empty<java.util.concurrent.Executor>()
    override fun newWebSocketBuilder() = throw UnsupportedOperationException()
}

/**
 * Create an HttpHelper with a fake HTTP client for unit testing.
 *
 * Uses reflection to inject the FakeHttpClient since httpClient is @PublishedApi internal.
 */
fun createTestHelper(config: EdgeQuakeConfig = EdgeQuakeConfig()): Pair<HttpHelper, FakeHttpClient> {
    val helper = HttpHelper(config)
    val fakeClient = FakeHttpClient()

    // Inject fake client via reflection
    val field = HttpHelper::class.java.getDeclaredField("httpClient")
    field.isAccessible = true
    field.set(helper, fakeClient)

    return Pair(helper, fakeClient)
}

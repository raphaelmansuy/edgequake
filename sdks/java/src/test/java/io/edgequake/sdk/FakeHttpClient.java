package io.edgequake.sdk;

import io.edgequake.sdk.internal.HttpHelper;

import javax.net.ssl.SSLContext;
import javax.net.ssl.SSLParameters;
import javax.net.ssl.SSLSession;
import java.lang.reflect.Field;
import java.net.Authenticator;
import java.net.CookieHandler;
import java.net.ProxySelector;
import java.net.http.HttpClient;
import java.net.http.HttpHeaders;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.ByteBuffer;
import java.time.Duration;
import java.util.*;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.Executor;
import java.util.concurrent.Flow;

/**
 * Fake HttpClient that intercepts HTTP calls and returns
 * pre-configured responses without network I/O.
 *
 * <p>Usage:</p>
 * <pre>{@code
 * var pair = FakeHttpClient.createTestHelper(config);
 * var http = pair[0];   // HttpHelper
 * var fake = pair[1];   // FakeHttpClient
 *
 * fake.respondWith("{\"status\":\"healthy\"}");
 * var result = new HealthService(http).check();
 * assertEquals("healthy", result.status);
 * }</pre>
 */
public class FakeHttpClient extends HttpClient {

    public record CapturedRequest(String method, String uri, String body) {}

    private final List<CapturedRequest> captured = new ArrayList<>();
    private String responseBody = "{}";
    private int responseCode = 200;
    private Exception shouldThrow = null;

    public void respondWith(String json) {
        respondWith(json, 200);
    }

    public void respondWith(String json, int code) {
        this.responseBody = json;
        this.responseCode = code;
        this.shouldThrow = null;
    }

    public void respondWithError(int code) {
        respondWithError(code, "{\"error\":\"mock error\"}");
    }

    public void respondWithError(int code, String body) {
        this.responseBody = body;
        this.responseCode = code;
        this.shouldThrow = null;
    }

    public void throwOnSend(Exception ex) {
        this.shouldThrow = ex;
    }

    public CapturedRequest lastRequest() {
        return captured.get(captured.size() - 1);
    }

    public List<CapturedRequest> allRequests() {
        return Collections.unmodifiableList(captured);
    }

    public void clear() {
        captured.clear();
    }

    // ── HttpClient overrides ─────────────────────────────────────────

    @Override
    @SuppressWarnings("unchecked")
    public <T> HttpResponse<T> send(HttpRequest request,
                                     HttpResponse.BodyHandler<T> handler) throws java.io.IOException {
        // Capture request body
        String bodyStr = null;
        if (request.bodyPublisher().isPresent()) {
            var pub = request.bodyPublisher().get();
            var sb = new StringBuilder();
            pub.subscribe(new Flow.Subscriber<ByteBuffer>() {
                @Override public void onSubscribe(Flow.Subscription s) { s.request(Long.MAX_VALUE); }
                @Override public void onNext(ByteBuffer item) {
                    var arr = new byte[item.remaining()];
                    item.get(arr);
                    sb.append(new String(arr));
                }
                @Override public void onError(Throwable t) {}
                @Override public void onComplete() {}
            });
            try { Thread.sleep(10); } catch (InterruptedException ignored) {}
            bodyStr = sb.length() > 0 ? sb.toString() : null;
        }

        captured.add(new CapturedRequest(request.method(), request.uri().toString(), bodyStr));

        if (shouldThrow != null) {
            if (shouldThrow instanceof java.io.IOException ioe) throw ioe;
            throw new java.io.IOException(shouldThrow);
        }

        final var code = responseCode;
        final var body = responseBody;

        return (HttpResponse<T>) new HttpResponse<String>() {
            @Override public int statusCode() { return code; }
            @Override public HttpHeaders headers() {
                return HttpHeaders.of(
                    Map.of("content-type", List.of("application/json")),
                    (k, v) -> true);
            }
            @Override public String body() { return body; }
            @Override public java.net.URI uri() { return request.uri(); }
            @Override public HttpRequest request() { return request; }
            @Override public Optional<HttpResponse<String>> previousResponse() { return Optional.empty(); }
            @Override public Optional<SSLSession> sslSession() { return Optional.empty(); }
            @Override public Version version() { return Version.HTTP_1_1; }
        };
    }

    @Override
    public <T> CompletableFuture<HttpResponse<T>> sendAsync(HttpRequest request,
                                                             HttpResponse.BodyHandler<T> handler) {
        try {
            return CompletableFuture.completedFuture(send(request, handler));
        } catch (Exception e) {
            return CompletableFuture.failedFuture(e);
        }
    }

    @Override
    public <T> CompletableFuture<HttpResponse<T>> sendAsync(HttpRequest request,
                                                             HttpResponse.BodyHandler<T> handler,
                                                             HttpResponse.PushPromiseHandler<T> pushHandler) {
        return sendAsync(request, handler);
    }

    @Override public Optional<CookieHandler> cookieHandler() { return Optional.empty(); }
    @Override public Optional<Duration> connectTimeout() { return Optional.of(Duration.ofSeconds(30)); }
    @Override public Redirect followRedirects() { return Redirect.NEVER; }
    @Override public Optional<ProxySelector> proxy() { return Optional.empty(); }
    @Override public SSLContext sslContext() { return null; }
    @Override public SSLParameters sslParameters() { return new SSLParameters(); }
    @Override public Optional<Authenticator> authenticator() { return Optional.empty(); }
    @Override public Version version() { return Version.HTTP_1_1; }
    @Override public Optional<Executor> executor() { return Optional.empty(); }

    // ── Factory ──────────────────────────────────────────────────────

    /**
     * Create an HttpHelper with a fake HTTP client for unit testing.
     * Uses reflection to inject the FakeHttpClient into HttpHelper's private httpClient field.
     *
     * @return Object[]{HttpHelper, FakeHttpClient}
     */
    public static Object[] createTestHelper(EdgeQuakeConfig config) {
        var helper = new HttpHelper(config);
        var fake = new FakeHttpClient();

        try {
            Field field = HttpHelper.class.getDeclaredField("httpClient");
            field.setAccessible(true);
            field.set(helper, fake);
        } catch (Exception e) {
            throw new RuntimeException("Failed to inject FakeHttpClient", e);
        }

        return new Object[]{helper, fake};
    }
}

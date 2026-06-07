# Header Propagation in edgequake-llm

> **Issue**: [raphaelmansuy/edgequake#132](https://github.com/raphaelmansuy/edgequake/issues/132)
> **Status**: Implemented in v0.6.16

---

## Problem statement

B2B operators run EdgeQuake as a platform-in-a-platform. Their services communicate
via standardised HTTP headers for tracing, security, and cost attribution. When
EdgeQuake calls LLM providers, all custom headers are lost — breaking distributed
tracing, audit logs, and tenant-level cost allocation.

---

## Design

### Approach: builder-pattern `with_extra_headers`

Each provider struct gains a `extra_headers: HashMap<String, String>` field.
A builder method `with_extra_headers(headers)` returns a new instance with those
headers applied to every outgoing HTTP request.

```rust
// Before (today)
let provider = MistralProvider::from_env()?;

// After (v0.6.16)
let provider = MistralProvider::from_env()?
    .with_extra_headers([
        ("x-request-id", request_id),
        ("x-tenant-id", tenant_id),
    ]);

// The headers are forwarded on every API call made by `provider`
let response = provider.chat(&request).await?;
```

### Why builder, not trait

Adding `extra_headers` to the `LlmProvider`/`EmbeddingProvider` trait would be a
**breaking change** for every downstream implementor. The builder method is:
- Non-breaking (optional, additive).
- Idiomatic Rust (builder pattern).
- Type-safe (headers are validated at construction, not at call time).
- Documented on docs.rs via the struct field.

### Where headers are applied

Headers are injected in the `reqwest::RequestBuilder` chain, **before** the
authentication header, ensuring they cannot shadow `Authorization`:

```rust
fn apply_extra_headers(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    let mut b = builder;
    for (k, v) in &self.extra_headers {
        // Silently skip headers that would override auth or content-type.
        // These are controlled by the provider, not the caller.
        if RESERVED_HEADERS.contains(&k.to_lowercase().as_str()) {
            continue;
        }
        b = b.header(k, v);
    }
    b
}
```

---

## Security considerations

### SC-HEADER-01: Header injection / CRLF injection

Header values must not contain `\r` or `\n`. `reqwest` enforces this at the
`header()` call level and returns an `InvalidHeaderValue` error. Our `with_extra_headers`
constructor validates and **rejects** any header name or value containing control characters.

### SC-HEADER-02: Overriding reserved headers

The following headers are reserved and will be **silently dropped** from `extra_headers`:

| Header | Reason |
|---|---|
| `authorization` | Provider authentication — set by the provider itself |
| `content-type` | Set to `application/json` by the provider |
| `content-length` | Computed from the body |
| `host` | Set by reqwest from the URL |
| `user-agent` | Set by the provider for rate-limit identification |

Silently dropping (rather than erroring) is intentional: it allows operators to
pass their full request header set (including Authorization) without needing to
filter it first. The LLM provider's own auth is never overridden.

### SC-HEADER-03: Sensitive header leakage in logs

`extra_headers` values are **never logged** at DEBUG or TRACE level. Only header
**names** may appear in logs (not values), to prevent credential or token leakage.

### SC-HEADER-04: Header size limits

Some LLM API gateways reject requests with very large header sets. The provider
implementation does not enforce a maximum, but the operator should be aware that
total header size is typically limited to 8-16 KB by most HTTP/1.1 servers.

---

## Edge cases

### EC-HEADER-01: Empty extra_headers map

`with_extra_headers(HashMap::new())` is a no-op. The HTTP request is unchanged.

### EC-HEADER-02: Duplicate header names (case-insensitive)

HTTP headers are case-insensitive. If `extra_headers` contains both `X-Tenant-Id`
and `x-tenant-id`, the behaviour is implementation-defined by `reqwest`. Our
`with_extra_headers` validates and **deduplicates** using lowercase keys, keeping
the last value.

### EC-HEADER-03: Headers not forwarded to embedding sub-calls

When `embed_batched` splits an input list into multiple sub-batches, **each sub-batch**
uses the same `&self` reference, so `extra_headers` is automatically included in all
sub-batch requests. No additional plumbing is needed.

### EC-HEADER-04: Thread safety

`extra_headers` is stored as `HashMap<String, String>` (immutable after construction).
`MistralProvider` is `Send + Sync` because `HashMap` with immutable access is thread-safe.
All provider structs must uphold `Send + Sync` for use with async runtimes.

### EC-HEADER-05: Forwarding from EdgeQuake API layer

The typical integration in `edgequake-api`:

```rust
// In a request handler:
async fn handle_query(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    ...
) -> Response {
    // Extract headers to forward
    let extra: HashMap<String, String> = headers
        .iter()
        .filter(|(k, _)| is_forwardable(k.as_str()))
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let provider = state.llm_provider.with_extra_headers(extra);
    let result = provider.chat(&request).await?;
    ...
}

fn is_forwardable(name: &str) -> bool {
    // Forward only headers that make sense to propagate
    matches!(name.to_lowercase().as_str(),
        "x-request-id" | "x-correlation-id" | "x-tenant-id" | "traceparent" | "tracestate"
    )
}
```

### EC-HEADER-06: Providers without HTTP transport

The Mock provider ignores `with_extra_headers`. This is intentional — the mock is
a test double that never makes HTTP calls.

### EC-HEADER-07: Headers on retried requests

If the provider retries a failed request (not currently implemented), `extra_headers`
must be included on the retry. Since headers are stored on `&self`, this is automatic
as long as the retry uses `self.build_request(...)`.

---

## Implementation checklist

- [x] `MistralProvider::with_extra_headers()` (v0.6.16)
- [x] `OpenAICompatibleProvider::with_extra_headers()` (v0.6.16)
- [x] `AnthropicProvider::with_extra_headers()` (v0.6.17)
- [x] `GeminiProvider::with_extra_headers()` (v0.6.17)
- [x] `NvidiaProvider::with_extra_headers()` (v0.6.17)
- [x] CRLF injection validation
- [x] Reserved header filtering
- [x] No-log of header values
- [ ] Integration in `edgequake-api` workspace resolver (future)
- [ ] E2E test: headers forwarded to provider via mock server (future)

---

## Future: request-level (per-call) headers

v0.6.16 implements **provider-level** headers (set once on the provider, applied to all calls).
A future version could support **per-call** headers via a `RequestContext` parameter
passed to `chat()` / `embed()`. This would be a trait-breaking change and requires
a new major minor version (trait default method) or a separate `ContextualLlmProvider` trait.

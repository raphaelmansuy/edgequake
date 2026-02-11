# OODA Iteration 12 — Orient: Rust SDK

## Analysis

The Rust SDK follows idiomatic Rust patterns:

1. **Builder pattern** — `EdgeQuakeClient::builder().base_url(...).api_key(...).build()?`
2. **Borrowed resources** — `DocumentsResource<'a>` borrows `&'a EdgeQuakeClient`
3. **Arc<ClientInner>** — Client is `Clone + Send + Sync` via Arc
4. **Retry with backoff** — Exponential backoff for 429/5xx responses
5. **Feature-gated streaming** — SSE via optional `reqwest-eventsource`

## Architecture Decisions

- Used `reqwest 0.13` (latest) instead of 0.12 for best compatibility
- Used `thiserror 2` for derive-based error types
- Used `wiremock 0.6` for mock HTTP testing
- All methods are `async fn` using tokio runtime
- Resources borrow client with lifetime `'a` (zero-cost, no Arc overhead for resources)
- Error type provides `status_code()` and `is_retryable()` for consumer use

## Design Patterns

| Pattern          | Implementation                             |
| ---------------- | ------------------------------------------ |
| Builder          | `ClientBuilder` with fluent API            |
| Resource handles | `DocumentsResource<'a>`                    |
| Error mapping    | `Error::from_response()` async             |
| Retry            | Exponential backoff in `send_with_retry()` |
| Auth middleware  | Header injection in `send_once()`          |

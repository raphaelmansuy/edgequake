# OODA-32: Orient

## Analysis

The Rust SDK had good structure but was missing ~30 endpoints. All are simple REST calls using existing client HTTP methods (get, post, put, patch, delete). No new types needed for most — `serde_json::Value` used for flexible responses.

## Priority

Filled ALL non-streaming gaps in one iteration. Streaming endpoints (query/stream, chat/stream, graph/stream, websocket) are deferred as they need SSE/WS plumbing.

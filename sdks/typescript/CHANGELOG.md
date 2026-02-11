# Changelog

All notable changes to `@edgequake/sdk` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-02-11

### Added

- **Core client** with config-based initialization (`EdgeQuake` class)
- **21 resource classes**: Auth, Users, ApiKeys, Documents (+ PDF sub-resource), Query, Chat, Graph (+ Entities, Relationships), Conversations (+ Messages), Folders, Shared, Tenants, Workspaces, Tasks, Pipeline, Costs, Lineage, Chunks, Provenance, Settings, Models, Ollama
- **Transport layer** with native `fetch()` — zero runtime dependencies
- **Middleware system** for auth, tenant headers, logging
- **Retry middleware** with exponential backoff and jitter
- **SSE streaming** via `parseSSEStream()` async generator
- **WebSocket wrapper** via `EdgeQuakeWebSocket` async iterable
- **Paginator** class implementing `AsyncIterable` for paginated endpoints
- **Typed error hierarchy**: `EdgeQuakeError`, `NotFoundError`, `UnauthorizedError`, `RateLimitedError`, `ValidationError`, `ConflictError`, `NetworkError`, `TimeoutError`
- **Dual module output**: ESM (.js) + CJS (.cjs) + TypeScript declarations (.d.ts)
- **243 unit tests** with 98.52% line coverage
- **8 usage examples** covering all major features
- **CI/CD pipelines** for testing (Node 18/20/22) and npm publishing

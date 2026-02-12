# Changelog

All notable changes to the EdgeQuake Python SDK will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-02-12

### Added

- Complete API coverage (20+ resource namespaces)
- Streaming query support via Server-Sent Events (SSE)
- Multi-tenant authentication with workspace IDs
- Comprehensive examples (8 runnable scenarios)
- Full API documentation in `docs/` folder
- Authentication guide (`docs/AUTHENTICATION.md`)
- Streaming guide (`docs/STREAMING.md`)
- Enhanced README with resource namespaces table and troubleshooting

### Fixed

- Cursor-based pagination implementation (Phase 6 fix)
- Async client resource cleanup
- Error handling for edge cases in streaming
- Type hints for all public methods

### Changed

- Improved error messages for better debugging
- Updated documentation to match TypeScript SDK quality standard

## [0.1.0] - 2026-02-10

### Added

- Initial Python SDK release
- Basic CRUD operations for documents, queries, graphs
- Synchronous client (`EdgequakeClient`)
- Asynchronous client (`AsyncEdgequakeClient`)
- Authentication support (API key, JWT tokens)
- Basic test coverage

---

[1.0.0]: https://github.com/edgequake/edgequake/compare/python-sdk-v0.1.0...python-sdk-v1.0.0
[0.1.0]: https://github.com/edgequake/edgequake/releases/tag/python-sdk-v0.1.0

# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

## [v0.2.2] - 2026-02-13

### Changed

- Updated workspace version to 0.2.2
- Refactored embedding batch calculation to use `.div_ceil()` (clippy compliance)
- Fixed consecutive `str::replace` calls in build scripts (clippy compliance)
- Feature gating improvements for minimal builds (query, core, storage)
- All clippy warnings resolved; workspace is clean
- Full test suite run: all tests passing

## [v0.2.1] - 2026-02-12

### Fixed

- Fixed TypeScript build error in dashboard: removed non-existent `entity_type_count` property reference
- Set entity types count to 0 as placeholder until backend implementation is complete

## [v0.2.0] - 2026-02-12

- Visual feedback for tenant/workspace switching in the knowledge graph view
- Loading overlay with minimum 800ms duration during workspace/tenant transitions
- Toast notifications for tenant and workspace switch confirmation
- Early return guard for same tenant/workspace selection (no-op)
- Toast deduplication using IDs to prevent duplicate notifications
- Loading overlay now always appears during workspace/tenant switch, even for empty/fast workspaces
- Only one toast notification is shown per switch (no duplicates)
- No notification or reload when selecting the same tenant/workspace
- See [SDKs documentation](sdks/) and [SDK changelogs](sdks/python/CHANGELOG.md, sdks/typescript/CHANGELOG.md, etc.) for language-specific updates.

---

## SDKs

EdgeQuake provides official SDKs for multiple languages. See the following for details and changelogs:

- [Python SDK](sdks/python/README.md) ([Changelog](sdks/python/CHANGELOG.md))
- [TypeScript SDK](sdks/typescript/README.md) ([Changelog](sdks/typescript/CHANGELOG.md))
- [Other SDKs](sdks/) for C#, Go, Java, Kotlin, PHP, Ruby, Rust, Swift

---

For a full project history, see the [README.md](README.md) and documentation in [docs/].

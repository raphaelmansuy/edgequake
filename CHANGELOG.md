# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [Unreleased]

### Added
- Visual feedback for tenant/workspace switching in the knowledge graph view:
	- Loading overlay with minimum 800ms duration during workspace/tenant transitions
	- Toast notifications for tenant and workspace switch confirmation
	- Early return guard for same tenant/workspace selection (no-op)
	- Toast deduplication using IDs to prevent duplicate notifications

### Fixed
- Loading overlay now always appears during workspace/tenant switch, even for empty/fast workspaces
- Only one toast notification is shown per switch (no duplicates)
- No notification or reload when selecting the same tenant/workspace

### Browser-verified
- Overlay visible at all transition points
- Toasts appear only once per switch
- Graph state and entity count correct after transitions

---

## [0.1.0] - 2026-02-12

### Added

- Initial CHANGELOG.md with structure for tracking changes across Rust backend and Next.js frontend.

## [0.1.0] - 2026-02-12

## [0.1.0] - 2026-02-12

### Added

- Build version auto-increment and git metadata in health API and frontend dashboard.
- Efficient entity type count for dashboard KPI (Cypher aggregate, no O(N) fetch).
- Orphaned document recovery on backend restart ("stuck uploading" fix).
- Shiki code block: fallback for unsupported languages (e.g., `dafny`).

### Changed

- Dashboard entity type KPI now uses backend count, not frontend Set().size.
- PDF cancel endpoint now allows cancelling both `Pending` and `Processing` states.

### Fixed

- Documents stuck in uploading/pending after cancel or restart can now be reset or cancelled.
- Shiki error for unsupported languages no longer breaks rendering.

---

Older changes and migration notes can be found in the `logs/` directory and in the project documentation.

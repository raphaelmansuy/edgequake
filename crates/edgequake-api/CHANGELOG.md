# Changelog (edgequake-api)

All notable changes to the EdgeQuake API crate are tracked here. See the root CHANGELOG.md for workspace-wide changes.

## [Unreleased]

### Added

- CHANGELOG.md for API crate.

## [0.1.0] - 2026-02-12

### Added

- Health API now includes build version and git metadata.
- Entity type count endpoint for dashboard KPI.
- Orphaned document recovery logic on startup.
- PDF cancel endpoint supports both `Pending` and `Processing` states.

### Changed

- Entity type KPI now uses backend aggregate count.

### Fixed

- Stuck uploading/cancel state for documents after restart or cancel.

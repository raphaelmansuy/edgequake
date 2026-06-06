---
title: 'Changelog (docs)'
---

# Changelog (docs)

All notable changes to the EdgeQuake documentation are tracked here. See the root CHANGELOG.md for workspace-wide changes.

## [Unreleased]

### Added

- **edgequake/docs/migrations.md** — SQLx migration overview, immutability rules, troubleshooting.
- **edgequake/docs/migrations/038-source-ids-indexes.md** — Migration 038 rollout, FAQ, edge-case matrix, verification commands.

### Changed

- Added Sigma graph-viewer performance tuning guidance to
  `docs/operations/performance-tuning.md`.
- Recast `mission/05-improve-performance-sigma.md` as an accepted ADR with
  explicit decision rules, alternatives, consequences, and verification.
- Aligned pinned quickstart and release examples with the April 2026 release
  line, finalized as official version `0.10.1`.
- Promoted the official publish target to `0.10.1` after matching the exact CI
  formatting gate used on GitHub Actions.

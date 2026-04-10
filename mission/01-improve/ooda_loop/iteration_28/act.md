# OODA-28 Act: WHY + 15 Tests for safety_limits.rs

## Changes

- **safety_limits.rs**: Added WHY comment (defense-in-depth: token clamping, timeout enforcement, API key pre-check) with ASCII diagram
- Added 15 tests: config defaults, clamp upper/lower boundaries (tokens+timeout), within-range passthrough, strict/permissive presets, without_logging builder, check_api_key for local providers (pass) and missing OpenAI key (fail), default_model_for_provider for known/unknown/case-insensitive/aliases

## Evidence
- Tests: 1299 → 1314 (+15)
- Clippy: 0 warnings

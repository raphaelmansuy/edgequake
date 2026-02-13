# OODA Iteration 01 — Decide: Action Plan

**Date**: 2026-02-13  
**Decision**: Start with baseline validation — run all existing tests, count actual endpoint coverage

## Decided Actions

### Action 1: Run Python SDK Tests
- Run `pytest` on Python SDK to establish baseline pass/fail
- Capture test count, pass count, coverage %

### Action 2: Run TypeScript SDK Tests  
- Run `npm test` / `vitest` on TypeScript SDK
- Capture test count, pass count

### Action 3: Run Rust SDK Tests
- Run `cargo test` on Rust SDK
- Capture test count, pass count

### Action 4: Count Endpoint Coverage per SDK
- For each SDK, grep for API paths to count covered endpoints
- Create initial coverage matrix

### Action 5: Commit Baseline Assessment
- Commit OODA iteration 01 files
- Tag as `OODA-01: Baseline SDK assessment`

## Rationale
- Must establish factual baseline before making changes
- Running tests validates that SDKs are in working state
- Coverage counting reveals actual gaps vs. estimates

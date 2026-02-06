# OODA-12 Orient: Data Model Quality Assessment

## Root Analysis
The data model is fundamentally sound but lacks E2E structure validation tests. The unit tests
in each `_types.rs` file test serialization but don't verify the actual HTTP response shapes.

## Options Considered

### Option A: Refactor DocumentSummary into composed types
- **Pro**: Cleaner structure, fewer fields per struct
- **Con**: Breaks API contract, frontend needs updating
- **Verdict**: Rejected — too risky mid-iteration

### Option B: Create shared CostMetrics struct
- **Pro**: Eliminates field duplication
- **Con**: Requires updating all consumers
- **Verdict**: Deferred to OODA-19 (cleanup iteration)

### Option C: Add E2E data model validation tests (CHOSEN)
- **Pro**: Validates actual API responses, catches regressions, zero risk
- **Con**: None significant
- **Verdict**: Best signal-to-risk ratio

## First Principles Applied
- **SRP**: Each DTO has a single purpose ✅
- **DRY**: Cost fields duplicated in 3 places ⚠️ (acceptable for now, API stability > DRY)
- **Backwards Compatibility**: Optional fields preserve existing clients ✅
- **Validation**: Content validation already exists ✅

## Decision
Create `e2e_data_model.rs` with 18 tests covering:
- Request validation (empty, whitespace, missing fields)
- Response structure (all required fields present)
- Edge cases (unicode, special characters, metadata)
- 404 handling (non-existent documents)
- Pagination structure
- Cost estimation structure
- Deletion cascade counts

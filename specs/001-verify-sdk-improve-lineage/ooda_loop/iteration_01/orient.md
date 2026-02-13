# OODA Iteration 01 — Orient: Gap Analysis & Prioritization

**Date**: 2026-02-13  
**Focus**: Analyze findings vs. requirements, identify critical gaps

## First Principles Analysis

### SDK Maturity Tiers

```
Tier 1 (Production-Ready): Python, TypeScript, Rust
  → 22+ resource files, typed responses, async support
  → Need: Fill remaining ~10-15% API gaps, add lineage export tests

Tier 2 (Functional but Incomplete): Go, C#, Ruby
  → Monolithic structure, good test LOC, partial coverage
  → Need: Split services, add missing endpoints, metadata support

Tier 3 (Minimal): Java, Kotlin, PHP, Swift
  → Basic structure, <1500 test lines, missing lineage
  → Need: Major refactoring, add 50%+ endpoints, full metadata
```

## Gap Analysis

### Critical Gaps (Tier 1 SDKs)

1. **Python**: Missing `export_lineage()` tests, no cost budget tests
2. **TypeScript**: Need streaming E2E tests, WebSocket tests incomplete
3. **Rust**: Types module has only 10 type files vs 22 resource files — type defs may be incomplete

### Major Gaps (Tier 2 SDKs)

1. **Go**: Flat structure — `services.go` is likely monolithic, need to verify endpoint count
2. **C#**: `Services.cs` is monolithic — need to audit covered endpoints vs 133 routes
3. **Ruby**: `services.rb` monolithic — need endpoint audit

### Blockers (Tier 3 SDKs)

1. **Java/Kotlin**: Missing lineage, provenance, chunks, costs entirely
2. **PHP**: Missing lineage export, entity provenance
3. **Swift**: Missing metadata, lineage, costs, models endpoints

## Risk Assessment

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Tests pass locally but fail in CI | High | Medium | Use mock HTTP, no live backend dependency |
| Breaking API changes during SDK work | High | Low | Pin to current routes.rs snapshot |
| Monolithic refactoring breaks existing tests | Medium | Medium | Incremental changes, run tests after each |
| Missing types cause runtime errors | High | Medium | Validate against backend DTO types |

## Priority Matrix (Impact × Effort)

```
HIGH IMPACT, LOW EFFORT (Do First):
├── Run existing tests on Python/TypeScript/Rust (validate baseline)
├── Count actual endpoint coverage per SDK
├── Fix any broken tests in top-3 SDKs

HIGH IMPACT, HIGH EFFORT (Plan Carefully):
├── Add missing lineage/metadata to Java/Kotlin/Swift
├── Refactor monolithic services in C#/Go/Ruby/PHP
├── Create unified test framework

LOW IMPACT, HIGH EFFORT (Defer):
├── WebSocket tests (complex setup)
├── CI/CD pipeline setup for all SDKs
```

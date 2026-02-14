# OODA-30 Orient: Analysis of Coverage Matrix Findings

## Date: 2026-02-14

## Analysis

### Positive Signals
1. **2,011 total tests** across all 10 SDKs — strong baseline
2. **100% lineage coverage** — all 7 lineage endpoints in all SDKs
3. **TypeScript leads** with 78+ methods and 96% estimated coverage
4. **Go is 2nd strongest** with 73 methods and 93% estimated coverage
5. **All tests pass** — zero failures across any SDK

### Primary Coverage Gaps

Using First Principles: The highest-impact gaps are where core SDKs (Python, Rust) lag behind secondary SDKs.

#### Python (Core SDK — should lead, currently lags)
- Missing: Tenants (5), Workspaces (9), Folders (4), Models (6), Settings (2), Costs (5) = **31 endpoints**
- Root cause: Python's resources are bundled (auth includes users+api-keys, operations includes tasks+pipeline)
- Impact: Python users can't manage multi-tenancy or models without raw HTTP calls

#### Rust (Core SDK — should lead, currently lags)
- Missing: Tenants (5), Workspaces (9), Costs (5), Models (6), Settings (2), Folders (4) = **31 endpoints**
- Root cause: Rust SDK focuses on document/graph/query core; infrastructure endpoints deferred
- Impact: Rust users need raw requests for administration

#### PHP/Ruby (Lowest test counts)
- PHP: 106 tests for 22 methods = ~4.8 tests/method
- Ruby: 109 tests for 22 methods = ~5.0 tests/method
- Compare: Go has 216 tests for 73 methods = ~3.0 tests/method (but more comprehensive edge cases within)
- PHP/Ruby are actually well-tested per-method, just have fewer methods

### Risk Assessment

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Python users hit multi-tenancy gap | High | Medium | Add Tenants/Workspaces resources |
| Rust users need admin features | Medium | Low | Add infrastructure endpoints |
| PHP/Ruby fall behind in features | Medium | Medium | Add missing services iteratively |
| Conversation bulk ops inconsistent | Low | Low | Not commonly used in SDK context |

## Recommendation

Focus next iterations on:
1. **OODA-31**: Python SDK — add Tenants + Workspaces + Settings resources + tests
2. **OODA-32**: Python SDK — add Models + Costs + Folders resources + tests
3. **OODA-33**: Rust SDK — add Tenants + Workspaces + Settings services
4. **OODA-34+**: PHP/Ruby — add Models + Settings services

# OODA-30 Observe: SDK Coverage Matrix Assessment

## Date: 2026-02-14

## Observation Summary

All 10 SDKs have been audited for test counts, service method counts, and API endpoint coverage.

## Test Counts (Validated)

| SDK        | Tests | Status  | Runner    |
|------------|------:|---------|-----------|
| Python     |   520 | ✅ Pass | pytest    |
| TypeScript |   288 | ✅ Pass | vitest    |
| Rust       |   156 | ✅ Pass | tokio     |
| Java       |   157 | ✅ Pass | JUnit 5   |
| Kotlin     |   155 | ✅ Pass | JUnit 5   |
| C#         |   154 | ✅ Pass | xUnit     |
| Swift      |   150 | ✅ Pass | XCTest    |
| Go         |   216 | ✅ Pass | testing   |
| PHP        |   106 | ✅ Pass | PHPUnit   |
| Ruby       |   109 | ✅ Pass | Minitest  |
| **Total**  | **2,011** |     |           |

## Service Method Counts

- Python: 7 resource modules, 27 public methods
- TypeScript: 21 resource modules, 78+ public methods (most comprehensive)
- Rust: ~34 public methods across modules
- Java: 20 separate service classes, 38 public methods
- Kotlin: 1 services file, 29 public methods
- C#: 1 services file, 24 public methods
- Swift: 1 services file, 33 public methods
- Go: 22 service types, 73 public methods (2nd most comprehensive)
- PHP: 1 services file, 22 public methods
- Ruby: 1 services file, 22 public methods

## Lineage Coverage

100% across all 10 SDKs — 7/7 endpoints implemented and tested.

## Key Gaps Identified

1. **Python**: Missing Tenants, Workspaces, Folders, Models, Settings, Costs as separate resources
2. **Rust**: Missing Tenants, Workspaces, Costs, Models, Settings, Folders
3. **PHP/Ruby**: Lower test counts vs. method surface (need more tests)
4. **Conversation Bulk Ops**: Only TypeScript has complete coverage
5. **Ollama Emulation**: Only TypeScript implements (lower priority)
6. **WebSocket**: Only TypeScript has partial support (SSE preferred)

# OODA Iteration 02 — Orient: Critical Issues Analysis

**Date**: 2026-02-13

## Priority Assessment

### P0 — Fix Broken Builds (Java, Kotlin, PHP)

1. **PHP**: `HealthService` class not found → autoloading issue. The `Services.php` likely defines all services in one file but tests reference individual class names.
2. **Java**: JDK 17 target in pom.xml but JDK 8 on system. Need to either update JDK path or adjust target.
3. **Kotlin**: Same JDK issue plus Jackson type ambiguity.

### P1 — Improve Low Coverage SDKs

The SDKs with fewest tests relative to API surface:
- Rust: 55 tests (but 22 resource files — may be well-structured)
- Ruby: 59 tests
- PHP: 7 passing (55 broken)
- Swift: 70 tests  

### P2 — Fill API Gaps

Need actual endpoint-by-endpoint mapping for each SDK.

## Root Cause: PHP Issue

The PHP `UnitTest.php` tries to instantiate `EdgeQuake\HealthService` but Services.php likely uses a different namespace pattern. Need to verify class structure.

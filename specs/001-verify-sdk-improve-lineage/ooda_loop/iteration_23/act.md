# OODA-23: Act — Kotlin SDK lineage tests

## Changes
- Added 33 lineage tests to `UnitTest.kt` (122 → 155 total)
  - 9 LineageService endpoint tests (all 7 endpoints + URL encoding + error)
  - 19 LineageModels field tests (all 19 data classes)
  - 5 edge case tests (null defaults, empty collections, client accessor)

## Evidence
```
Tests run: 155, Failures: 0, Errors: 0, Skipped: 0
BUILD SUCCESS
```

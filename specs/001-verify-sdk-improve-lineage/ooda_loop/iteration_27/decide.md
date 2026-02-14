# OODA-27 Decide: PHP SDK Lineage Implementation

## Actions

1. Add `LineageService` class to `Services.php` — 7 methods, `rawurlencode()` for URL safety
2. Wire `LineageService` into `Client.php` — add `public readonly LineageService $lineage`
3. Add 15 unit tests to `UnitTest.php` covering all 7 methods + edge cases + error handling
4. Update `testClientInitializesAllServices` to assert all 17 services

## Priority: HIGH

- Closes metadata/lineage gap for PHP SDK
- Follows established pattern — minimal risk
- Estimated: 15 minutes

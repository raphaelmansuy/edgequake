# OODA-27 Orient: PHP SDK Lineage Analysis

## Pattern

PHP SDK uses the simplest pattern of all SDKs — all methods return `array`.
No typed models needed (unlike Swift, C#, Java, Kotlin).

## Approach

- Add `LineageService` class to Services.php with 7 methods
- Wire into Client.php (16 → 17 services)
- Add tests using MockHttpHelper pattern (simple, proven)
- URL encoding: `rawurlencode()` for entity names with spaces/special chars

## Risks

- URL path mismatches between test expectations and implementation
- `exportLineage()` must return `string` via `getRaw()` (not `array`)
- Special characters in entity names need proper encoding

## PHP-Specific Considerations

- No need for typed response models (arrays are sufficient)
- PHPUnit assertions: `assertSame`, `assertInstanceOf`, `assertIsArray`, `assertStringContainsString`
- Error handling automatically via MockHttpHelper's `willReturn(json, status)` throwing ApiError

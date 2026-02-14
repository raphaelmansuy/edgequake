# OODA-28 Orient: Ruby SDK Lineage Analysis

## Pattern
Ruby SDK uses the same simple pattern as PHP — methods return Hash (parsed JSON).
`get_raw()` returns raw String for export endpoints.
URL encoding via `URI.encode_www_form_component()` (stdlib).

## Approach
- Add `LineageService` class to services.rb with 7 methods
- Wire into client.rb (16 → 17 services)
- Add 16 Minitest tests using MockHttpHelper pattern
- Ruby uses keyword arguments (`name:`, `id:`, `format:`)

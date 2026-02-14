# OODA-28 Decide: Ruby SDK Lineage Implementation

## Actions
1. Add `LineageService` class to `services.rb` — 7 methods, `URI.encode_www_form_component` for URL safety
2. Wire `LineageService` into `client.rb` — add to `attr_reader` and constructor
3. Add 16 Minitest tests to `unit_test.rb` covering all 7 methods + edge cases + error handling

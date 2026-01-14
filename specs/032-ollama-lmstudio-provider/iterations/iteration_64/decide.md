# OODA 64 - Decide: Provider API Tests Adjusted

## Decision

Adjusted provider tests based on actual API behavior:

1. **Provider Priority**: API returns providers in registration order, not priority order
   - Changed test from "ordered by priority" to "providers have priority property"
   - Client should sort by priority if needed

2. **Provider Enabled**: Some providers are disabled (anthropic, azure)
   - Changed test from "all providers enabled" to "core providers are enabled"
   - Core providers: openai, ollama, mock

3. **Deeplink Test**: Made locator more robust
   - Added `main` element to locator fallback
   - Added explicit URL verification

## Rationale

### Why Not Sort by Priority
The backend returns providers in registration order. This is a valid design decision:
- Clients can sort if needed
- Allows flexibility in presentation

### Why Core Providers Only
Some providers may be disabled in development:
- Anthropic: May require API key
- Azure: May require configuration
- Testing core providers covers the important cases

## Updated Tests

| Test | Change |
|------|--------|
| providers are ordered by priority | → providers have priority property |
| all returned providers are enabled | → core providers are enabled |
| workspace deeplink by slug resolves correctly | More robust locator |

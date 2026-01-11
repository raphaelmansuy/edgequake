# Task Log: Fix Default Model Regression

**Date:** 2026-01-11 14:41 UTC
**Branch:** feat/newproviders

## Actions

- Identified root cause: when provider specified without model (e.g., "ollama"), code defaulted to literal string "default" which Ollama doesn't recognize
- Added `default_model_for_provider()` helper to `ProviderFactory` in factory.rs
- Fixed non-streaming chat handler in chat.rs (~line 375)
- Fixed streaming chat handler in chat.rs (~line 674)
- Verified build compiles successfully
- Ran 397 tests - all pass
- Tested streaming chat with `provider=ollama` - correctly resolves to `gemma3:12b`

## Decisions

- Default models per provider: ollama→gemma3:12b, openai→gpt-4o-mini, lmstudio→gemma2-9b-it, mock→mock-model
- Fallback to gpt-4o-mini for unknown providers
- Workspace "default" references are correct behavior (not LLM model-related)

## Next Steps

- E2E test with WebUI to confirm full regression fix
- Consider adding tests for `default_model_for_provider()` helper

## Lessons/Insights

- Literal string "default" as model name is never valid for real providers
- Always resolve provider-specific defaults at the factory/handler level

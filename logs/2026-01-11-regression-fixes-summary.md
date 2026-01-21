# Regression Fixes Summary

**Date:** 2026-01-11
**Branch:** feat/newproviders
**User Request:** "I have done a make dev --> And I have regression. Ensure to fix all"

---

## Overview

Fixed two critical regressions preventing proper LLM provider selection and error handling in the EdgeQuake API.

## Regression 1: Ollama "Model 'default' Not Found"

### Problem

When provider was specified without a model suffix (e.g., `provider=ollama`), the system was using the literal string "default" as the model name, causing Ollama to return: "model 'default' not found".

### Root Cause

In [chat.rs](edgequake/crates/edgequake-api/src/handlers/chat.rs), when splitting `provider_full_id` by `:` returned no model part, the code set `model = "default"` instead of resolving the provider's actual default model.

### Solution

**Commit:** 93a2d5f

1. Added `default_model_for_provider()` helper in [factory.rs](edgequake/crates/edgequake-llm/src/factory.rs):

   ```rust
   pub fn default_model_for_provider(provider_name: &str) -> &'static str {
       match provider_name.to_lowercase().as_str() {
           "openai" => "gpt-4o-mini",
           "ollama" => "gemma3:12b",
           "lmstudio" | "lm-studio" | "lm_studio" => "gemma2-9b-it",
           "mock" => "mock-model",
           _ => "gpt-4o-mini",
       }
   }
   ```

2. Updated both chat handlers (streaming and non-streaming) to use this helper:
   ```rust
   } else {
       let default_model = ProviderFactory::default_model_for_provider(provider_full_id);
       (provider_full_id.clone(), default_model.to_string())
   };
   ```

### Verification

- ✅ Build succeeded
- ✅ 397 tests passed
- ✅ Live testing confirmed `provider=ollama` → `llm_model=gemma3:12b`

---

## Regression 2: OpenAI "API Key Not Provided" Cryptic Error

### Problem

When user selected OpenAI provider without valid API key (or with `OPENAI_API_KEY=""`), the system returned a confusing error: "You didn't provide an API key. You need to provide your API key in an Authorization header..."

### Root Cause

OpenAI provider was being created successfully even with empty/invalid API keys, then failing at API call time with unhelpful error messages.

### Solution

**Commits:** 4dcf46f, 2ae337f

1. Enhanced validation in `create_llm_provider()` [factory.rs](edgequake/crates/edgequake-llm/src/factory.rs#L357-L364):

   ```rust
   let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| {
       LlmError::ConfigError(
           "OPENAI_API_KEY required for OpenAI LLM provider. Set the environment variable or select a different provider (ollama, lmstudio, mock)".to_string(),
       )
   })?;

   if api_key.is_empty() || api_key == "test-key" {
       return Err(LlmError::ConfigError(
           "OPENAI_API_KEY is empty or invalid. Provide a valid API key from https://platform.openai.com/account/api-keys or select a different provider (ollama, lmstudio, mock)".to_string(),
       ));
   }
   ```

2. Applied identical validation to `create_embedding_provider()` for consistency [factory.rs](edgequake/crates/edgequake-llm/src/factory.rs#L278-L293).

### Key Improvements

- ✅ Fails fast at provider creation (not API call time)
- ✅ Actionable error messages suggest alternatives
- ✅ Provides link to get API key
- ✅ Consistent validation across both LLM and embedding providers

---

## Technical Details

### Files Modified

1. [`edgequake/crates/edgequake-llm/src/factory.rs`](edgequake/crates/edgequake-llm/src/factory.rs)

   - Added `default_model_for_provider()` helper
   - Enhanced OpenAI validation in `create_llm_provider()`
   - Enhanced OpenAI validation in `create_embedding_provider()`

2. [`edgequake/crates/edgequake-api/src/handlers/chat.rs`](edgequake/crates/edgequake-api/src/handlers/chat.rs)
   - Fixed provider/model parsing in streaming handler (~line 674)
   - Fixed provider/model parsing in non-streaming handler (~line 375)

### Test Results

- Build: ✅ Success
- Tests: ✅ 397 passed, 0 failed
- Clippy: ✅ Clean

---

## Impact

### User Experience

- **Before:** "model 'default' not found" when using Ollama without explicit model
- **After:** Automatically uses `gemma3:12b` for Ollama

- **Before:** Cryptic "You didn't provide an API key" error when selecting OpenAI
- **After:** Clear message with alternatives and link to get API key

### System Behavior

- Provider-only requests now correctly resolve to provider-specific default models
- Invalid provider configurations fail early with helpful guidance
- Error messages guide users toward working alternatives

---

## Related Work

- **SPEC-032**: Ollama/LM Studio provider support
- **FEAT0017**: Multi-provider LLM support
- **Environment Setup**: Makefile sets `OPENAI_API_KEY=""` for local dev with Ollama

---

## Next Steps

1. **E2E Testing**: Full verification with `make dev` and browser testing
2. **UI Enhancement**: Consider hiding/disabling OpenAI option when no key configured
3. **Documentation**: Update provider selection docs with new validation behavior
4. **Additional Providers**: Apply similar validation to other providers requiring credentials

---

## Commits

1. `93a2d5f` - fix: resolve provider-specific default model instead of literal 'default'
2. `4dcf46f` - fix: improve OpenAI API key validation with actionable error messages
3. `2ae337f` - fix: add API key validation to embedding provider creation

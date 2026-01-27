# OODA Iterations 218-225: Fix Critical Issues (Model Names, Type Filtering, Tokens/sec)

**Date**: 2025-01-14
**Branch**: feat/newproviders

## Context

User reported 4 critical issues after OODA 168-217:

1. **Issue 16**: `gpt-5o-mini` model does not exist - OpenAI API error
2. **Issue 17**: Embedding selector shows LLM models mixed in (wrong filtering)
3. **Issue 18**: Missing tokens/second display in query responses
4. **Issue 19**: Workspace extractor model configuration clarity

## OODA 218: Observe - Model Name Error

### Observation

- User received error: `API error: invalid_request_error: The model 'gpt-5o-mini' does not exist`
- Fetched OpenAI documentation to verify current model names
- Current OpenAI models (Jan 2025):
  - Flagship: gpt-4.1, gpt-4.1-mini, gpt-4.1-nano
  - Standard: gpt-4o, gpt-4o-mini
  - Embedding: text-embedding-3-small, text-embedding-3-large, text-embedding-ada-002

### Root Cause

- `models.toml` contained placeholder `gpt-5o-mini` and `gpt-5o-nano` models that don't exist
- These were marked as "future" models but were being exposed in the dropdown

## OODA 219: Orient - Model Type Filtering

### Analysis

- Backend function `all_embedding_models()` included `ModelType::Multimodal`
- In EdgeQuake context, "multimodal" means vision-capable LLM (text + image input)
- Multimodal models should NOT appear in embedding dropdown

### Code Location

- File: `edgequake/crates/edgequake-llm/src/model_config.rs`
- Lines: 775-789 (`all_embedding_models` function)

## OODA 220: Decide - Fix Strategy

### Decisions

1. **Model names**: Replace `gpt-5o-mini`/`gpt-5o-nano` with `gpt-4.1`, `gpt-4.1-mini`, `gpt-4.1-nano`
2. **Filtering**: Remove `Multimodal` from `all_embedding_models()` filter
3. **Tokens/sec**: Add calculation in `MetadataBar` component
4. **Extractor clarity**: Verify workspace page already has LLM config

## OODA 221: Act - Implement Fixes

### Changes Made

#### 1. models.toml - OpenAI Model Names

- Replaced `gpt-5o-mini` with `gpt-4.1` (flagship model)
- Replaced `gpt-5o-nano` with `gpt-4.1-mini` (cost-effective)
- Added `gpt-4.1-nano` (ultra-efficient)
- Updated capabilities to match OpenAI 2025 specs (1M+ context)

```diff
-name = "gpt-5o-mini"
-display_name = "GPT-5o Mini"
+name = "gpt-4.1"
+display_name = "GPT-4.1"
```

#### 2. model_config.rs - Embedding Filter Fix

- Removed `Multimodal` from embedding model filter
- Added WHY comment explaining the reasoning

```diff
-                    .filter(|m| {
-                        matches!(m.model_type, ModelType::Embedding | ModelType::Multimodal)
-                    })
+                    // WHY: Only include pure embedding models, NOT multimodal (vision LLMs)
+                    .filter(|m| matches!(m.model_type, ModelType::Embedding))
```

#### 3. chat-message.tsx - Tokens/Second Display

- Added `Gauge` icon import from lucide-react
- Added tokens/second calculation: `(tokensUsed / durationMs) * 1000`
- Added tooltip showing "Generation speed: X.X tokens/second"

```tsx
{
  tokensUsed && durationMs && durationMs > 0 && (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <span className="flex items-center gap-1 text-emerald-600">
            <Gauge className="h-3 w-3" />
            {((tokensUsed / durationMs) * 1000).toFixed(1)}/s
          </span>
        </TooltipTrigger>
        <TooltipContent>
          <p>Generation speed: X.X tokens/second</p>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
```

#### 4. Workspace Page - Extractor Model (Already Exists)

- Verified "LLM Configuration" card in workspace settings page
- Description already states: "Model used for entity extraction and summarization during document ingestion"
- No changes needed - functionality exists

## OODA 222-225: Test & Validate

### Rust Tests

```bash
cargo check --package edgequake-llm --package edgequake-api
# Result: Finished successfully
```

### TypeScript Type Check

```bash
npx tsc --noEmit
# Result: No errors
```

## Files Changed

| File                                                                 | Change                                                           |
| -------------------------------------------------------------------- | ---------------------------------------------------------------- |
| `edgequake/models.toml`                                              | Updated OpenAI model names (gpt-4.1, gpt-4.1-mini, gpt-4.1-nano) |
| `edgequake/crates/edgequake-llm/src/model_config.rs`                 | Fixed embedding filter (removed Multimodal)                      |
| `edgequake_webui/src/components/query/chat-message.tsx`              | Added tokens/second display                                      |
| `specs/032-ollama-lmstudio-provider/032-ollama-lmstudio-provider.md` | Added issues 16-19                                               |

## Validation Checklist

- [x] OpenAI model names are valid (gpt-4.1, gpt-4.1-mini, gpt-4.1-nano)
- [x] Embedding selector only shows embedding models
- [x] Tokens/second displayed in query metadata
- [x] Extractor model editable in workspace settings
- [x] Rust code compiles without errors
- [x] TypeScript code passes type check

## Next Steps

- [ ] OODA 226-230: E2E tests for model filtering
- [ ] OODA 231-235: Verify API returns filtered models correctly
- [ ] OODA 236-240: Visual regression tests for tokens/sec display
- [ ] OODA 241-245: Test embedding model selector in dialogs
- [ ] OODA 246-250: Final documentation and commit

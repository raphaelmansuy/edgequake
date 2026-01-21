# OODA Iterations 171-180: E2E Tests for Dialog Model Selection & Deeplinks

## Objective

Add comprehensive E2E tests for Focus 1, 2, and 6 (Dialog Model Selection and Deeplink Routes).

## Iteration 171-175: Dialog Model Selection Tests

**Focus:** Test tenant and workspace creation dialogs with model selection

### Tests Added:

1. **workspace selector button exists in header** - Verifies the workspace selector is visible
2. **create tenant dialog opens from dropdown** - Tests "Create New Tenant" menu item visibility
3. **create workspace dialog opens from dropdown** - Tests "Create New Workspace" menu item visibility
4. **tenant creation API accepts model configuration** - API test for creating tenant with LLM/embedding model config
5. **workspace creation API accepts model configuration** - API test for creating workspace with full model config

### API Fields Tested:

- `default_llm_provider` / `llm_provider`
- `default_llm_model` / `llm_model`
- `default_embedding_provider` / `embedding_provider`
- `default_embedding_model` / `embedding_model`
- `embedding_dimension`

## Iteration 176-180: Deeplink Route Tests

**Focus:** Test all deeplink routes from OODA 169

### Tests Added:

1. **deeplink /w/[slug]/documents redirects correctly** - Tests documents deeplink
2. **deeplink /w/[slug]/graph redirects correctly** - Tests graph deeplink
3. **deeplink /w/[slug]/query loads query page** - Tests query deeplink
4. **deeplink /w/[slug]/settings redirects to workspace settings** - Tests settings deeplink

### Validation Criteria:

- Routes either redirect to the correct page or show "Workspace Not Found" for invalid slugs
- URL patterns match expected format after redirect
- No JavaScript errors during navigation

## Files Modified

- `edgequake_webui/e2e/spec032-provider-integration.spec.ts` (+103 lines)

## Test Categories

| Category   | Tests Added | Focus       |
| ---------- | ----------- | ----------- |
| Dialog UI  | 3           | Focus 1 & 2 |
| Dialog API | 2           | Focus 1 & 2 |
| Deeplinks  | 4           | Focus 6     |
| **Total**  | **9**       | —           |

## Next Steps

- OODA 181-190: API Explorer enhancements and response time tests
- OODA 191-200: Query response lineage display (show provider/model used)

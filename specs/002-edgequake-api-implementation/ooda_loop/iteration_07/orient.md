# Iteration 07: Orient

## Date: 2025-07-25

## Analysis

### Root Causes

1. **No userId in SDK config** — The EdgeQuake API requires `X-User-ID` header for user-scoped endpoints (chat, conversations). The SDK's config and middleware pipeline had no mechanism to send this header.

2. **Types designed from spec, not from code** — Initial SDK types were inferred from API route patterns and naming conventions. Actual Rust handler response structures differ significantly (different field names, additional fields, nested objects).

3. **Paginated responses not unwrapped** — Tenant and workspace list endpoints return paginated wrappers `{items: [], total, offset, limit}`. The SDK's resource methods returned the raw wrapper instead of extracting the `items` array.

4. **Chat response format diverged from OpenAI convention** — The Rust API uses its own response format (`content` field, discriminated union stream events) rather than OpenAI-compatible format assumed by the initial SDK.

### Approach Chosen

**Systematic type alignment** — Read each Rust handler's response type, compare field-by-field with SDK TypeScript types, and update SDK types to match exactly. This ensures the SDK accurately represents the API contract.

**userId as first-class config** — Add `userId` to `EdgeQuakeConfig`, `ResolvedConfig`, and the tenant middleware pipeline. Read from `EDGEQUAKE_USER_ID` env var as fallback.

**E2E test expansion** — Create dedicated test files for resource groups not yet covered (tenants, workspaces, tasks, pipeline, settings, models) to validate the corrected types against the live API.

### Risks & Mitigations

| Risk                                    | Mitigation                                        |
| --------------------------------------- | ------------------------------------------------- |
| Breaking changes to existing consumers  | No published package yet — internal types only    |
| Chat stream format changes              | Discriminated union with exhaustive type checking |
| Workspace metrics history table missing | Test catches error gracefully, logs it            |
| Document delete timing flake            | Test already handles 409 Conflict gracefully      |

# OODA Iteration 15 — Orient

## Root Cause Analysis

The fundamental issue is a **design-implementation gap**: the SDKs were designed with OpenAI-compatible chat format (`messages` array with role/content), but the EdgeQuake backend uses its own native format (`message` singular string with conversation threading).

### Priority Matrix

| Issue                        | Impact | Effort | Priority |
|------------------------------|--------|--------|----------|
| Chat API mismatch (7 SDKs)   | HIGH   | MED    | P0       |
| E2E tenant/user skips (25)   | HIGH   | LOW    | P0       |
| .gitignore missing (4 SDKs)  | LOW    | LOW    | P1       |
| OpenAPI spec gaps             | MED    | MED    | P2       |
| Rust SDK missing user_id      | MED    | LOW    | P1       |

### Approach: SDK-First (Fix SDKs to match API)

The backend API is the source of truth. The SDKs must be updated to match.

**Chat types strategy**:
1. Replace `messages: Vec<ChatMessage>` with `message: String`
2. Replace OpenAI-style response with EdgeQuake response shape
3. Keep the method simple: `chat.complete("message")` or `chat.completions("message")`
4. Add optional fields: mode, conversation_id, max_tokens, temperature, provider, model

**E2E strategy**:
1. Use hardcoded default tenant/user IDs in E2E setup
2. Remove all "skip if no tenant" guards
3. Test conversation CRUD, folder CRUD, chat in every SDK

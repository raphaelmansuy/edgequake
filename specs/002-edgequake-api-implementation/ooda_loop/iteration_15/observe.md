# OODA Iteration 15 — Observe

## API Surface Audit

### Chat API Mismatch (CRITICAL)

**Actual API** (`POST /api/v1/chat/completions`):
- Request: `{"message": "...", "stream": false, "mode": "hybrid", ...}`
- Response: `{"conversation_id": "uuid", "content": "...", "sources": [...], "stats": {...}}`

**SDK implementations** (WRONG for 7/10 SDKs):
- Python: sends `{"messages": [{"role":"user","content":"..."}], "model":"...", "stream": false}`
- Go: sends `{"messages": [{"role":"user","content":"..."}]}`
- Rust: sends `{"messages": [{"role":"user","content":"..."}]}`
- Java: sends `{"messages": [{"role":"user","content":"..."}]}`
- Kotlin: sends `{"messages": [{"role":"user","content":"..."}]}`
- TypeScript: MAY be correct (need to verify)

**SDK implementations** (CORRECT):
- PHP: sends `completions('message text')` — correct!
- Ruby: sends `completions(message: 'text')` — correct!
- Swift: sends `ChatCompletionRequest(message: "text")` — correct!
- C#: sends `CompletionsAsync("text")` — correct!

### E2E Test Skips (25 total across 10 SDKs)

| SDK        | Skips | Reason                                       |
|------------|-------|----------------------------------------------|
| TypeScript | 14    | conversations, folders, sharing need tenant   |
| Python     | 1     | chat catches 422 and skips                    |
| PHP        | 2     | conversations, folders hardcoded skip         |
| Ruby       | 2     | conversations, folders skip unless env vars   |
| Swift      | 2     | conversations, folders XCTSkip                |
| Kotlin     | 2     | conversations, folders Assumptions.assumeTrue |
| Java       | 2     | conversations, folders Assumptions.assumeTrue |
| Go         | 0     | uses t.Skip() on failure (graceful)           |
| Rust       | 0     | no conversation/folder tests at all           |
| C#         | 0     | uses early return (silent skip)               |

### .gitignore Audit

Missing .gitignore: Go ✅(just created), Java ✅(just created), Kotlin ✅(just created), Python, Ruby, Rust, Swift

### OpenAPI Spec Gaps

Missing from `openapi.rs`:
- Chat endpoints (chat_completion, chat_completion_stream)
- Conversation endpoints (list, create, get, update, delete, messages, share)
- Folder endpoints (list, create, update, delete)
- Tenant endpoints (create, list, get, update, delete)
- Workspace endpoints (create, list, get, update, delete, stats)
- Pipeline endpoints (status, queue-metrics, cancel)
- Cost endpoints (summary, history, budget)
- Task endpoints (list, get, cancel, retry)
- Document advanced endpoints (upload file, pdf, scan, reprocess)
- Shared conversation endpoint
- Lineage endpoints
- Settings/providers endpoints

### Default Tenant/User IDs

Confirmed working for conversations/folders:
- Tenant: `00000000-0000-0000-0000-000000000002` (slug: "default")
- User: `00000000-0000-0000-0000-000000000001` (username: "default_user")

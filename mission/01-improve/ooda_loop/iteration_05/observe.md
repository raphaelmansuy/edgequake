# Observe

- User rejected `iteration_04` because broadening `200 OK` assertions to allow `500` masked the underlying issue instead of fixing it.
- The disputed changes are in `edgequake/crates/edgequake-api/tests/e2e_query_http_workspace.rs` for the OpenAI-configured workspace query tests.
- Current shell inspection shows `OPENAI_API_KEY` is present in the agent environment, which means isolated reruns can pass under conditions different from the earlier failure.
- `tests/common/mod.rs` already contains `clear_provider_detection_env()` specifically to make provider-related tests deterministic, but `e2e_query_http_workspace.rs` does not call it.
- `create_workspace_with_embedding_config()` stores a mock LLM plus a workspace-specific embedding config, so the intended behavior for these tests is: preserve workspace config, fall back safely when provider creation is unavailable, and still return `200 OK`.# Observe — Iteration 05

Date: 2026-04-10
Mission file re-read: `mission/01-improve.md`

## Findings

Production code contains `serde_json::to_value().unwrap()` in handler paths:

| File                                       | Lines    | Risk                               |
| ------------------------------------------ | -------- | ---------------------------------- |
| `handlers/workspaces/bulk_ops/mod.rs`      | 254, 292 | Task serialization in request path |
| `handlers/documents/recovery/stuck.rs`     | 192      | Recovery re-enqueue                |
| `handlers/documents/recovery/reprocess.rs` | 244, 315 | Reprocess task creation            |
| `handlers/documents/query/scan.rs`         | 206      | Scan task creation                 |
| `handlers/documents/upload/text_upload.rs` | 245      | Upload task creation               |

These serialize simple `#[derive(Serialize)]` structs with no custom serializers—technically infallible.
However, using bare `.unwrap()` violates defensive coding principles and hides intention.

## Risk Assessment

- **Severity**: Low (structs always serialize)
- **Signal**: High (bare unwrap masks WHY it's safe)
- **Fix cost**: Minimal (unwrap → expect with rationale)

# Edge Case Registry: edgequake-llm

> This document is the authoritative registry of all edge cases, invariants,
> and safety properties for the `edgequake-llm` crate. Every entry has a
> unique ID (EC-*) and a corresponding test or test plan.

---

## Invariants (must never be violated)

| ID | Invariant | Test |
|---|---|---|
| INV-01 | `embed(&[]) → Ok(vec![])` without API call | `test_embed_empty_input` |
| INV-02 | `embed(texts).len() == texts.len()` | `test_embed_output_count_matches_input` |
| INV-03 | `max_batch_size() ≤ provider_hard_limit` | Per-provider constant test |
| INV-04 | Providers are `Send + Sync` | Compile-time (`static_assertions`) |
| INV-05 | `extra_headers` values never appear in logs | Audit / manual review |
| INV-06 | Authorization header is never overridden by `extra_headers` | `test_reserved_headers_not_overridden` |

---

## Embedding edge cases

### EC-EMBED-01: Empty input slice
**Trigger**: `embed(&[])`.
**Expected**: `Ok(vec![])`, no HTTP call.
**Risk**: Some naive providers attempt to `POST` an empty array, which some APIs reject.
**Status**: Tested in all providers.

### EC-EMBED-02: Single-item input
**Trigger**: `embed(&["hello"])`.
**Expected**: `Ok(vec![vec![f32; dim]])`.
**Status**: Tested.

### EC-EMBED-03: Batch exactly at size limit
**Trigger**: `embed` with exactly `max_batch_size()` inputs.
**Expected**: Sent as a single batch (no split), returns `max_batch_size()` vectors.
**Risk**: Off-by-one error in split logic sends one too many.
**Status**: Tested for Mistral (512 inputs).

### EC-EMBED-04: Batch one over size limit
**Trigger**: `embed` with `max_batch_size() + 1` inputs.
**Expected**: Split into two batches: [max_batch_size, 1].
**Status**: Tested via `embed_batched` unit tests.

### EC-EMBED-05: All inputs identical
**Trigger**: `embed(&["foo"; 100])`.
**Expected**: 100 identical vectors returned (API may cache; output order must match input).
**Risk**: Provider returns deduplicated results; we must re-expand.
**Status**: Mistral sorts by `index` field in response → order guaranteed.

### EC-EMBED-06: Non-UTF-8 / special characters
**Trigger**: Inputs containing null bytes, surrogates, or very long Unicode sequences.
**Expected**: Provider returns an embedding or a clear error (not a panic).
**Risk**: JSON serialisation of null bytes can corrupt the request body.
**Status**: `serde_json` serialises Rust `String` (guaranteed UTF-8). Null bytes are
         legal UTF-8 but some providers reject them. Propagated as `ApiError`.

### EC-EMBED-07: Response order mismatch
**Trigger**: Provider returns embeddings in non-input order.
**Expected**: Output is re-ordered to match input order.
**Implementation**: Mistral response includes `index` field; vectors are sorted by index
                  before returning. OpenAI guarantees order in spec.
**Status**: Tested.

---

## Batch splitting edge cases

### EC-BATCH-01 through EC-BATCH-07
See [BATCH_SIZE_LIMITS.md](./BATCH_SIZE_LIMITS.md).

---

## Header propagation edge cases

### EC-HEADER-01 through EC-HEADER-07
See [HEADER_PROPAGATION.md](./HEADER_PROPAGATION.md).

---

## Authentication edge cases

### EC-AUTH-01: Missing API key
**Trigger**: Provider constructed without API key.
**Expected**: `LlmError::AuthError` at construction or first API call.
**Current**: `MistralProvider::from_env()` returns `Err` if `MISTRAL_API_KEY` is unset.
**Risk**: Providers that lazily validate (detect only at call time) may fail in unexpected places.

### EC-AUTH-02: Expired API key
**Trigger**: API key is set but expired / revoked.
**Expected**: Provider returns `LlmError::AuthError` (HTTP 401/403).
**Current**: Propagated as `LlmError::ApiError` with status code in message.
**Note**: A future improvement could parse the status code and return a typed
         `LlmError::AuthError` variant for 401/403 specifically.

### EC-AUTH-03: Anthropic dual-key fallback
**Scenario**: Both `ANTHROPIC_API_KEY` and `ANTHROPIC_AUTH_TOKEN` set.
**Expected**: `ANTHROPIC_API_KEY` takes precedence; `ANTHROPIC_AUTH_TOKEN` used as fallback.
**Risk**: Empty string `ANTHROPIC_API_KEY=""` must be treated as "not set".
**Status**: Fixed (see user memory: Anthropic-compatible endpoints).

---

## Network edge cases

### EC-NET-01: Request timeout
**Trigger**: Provider API takes > N seconds to respond.
**Expected**: `LlmError::NetworkError` with timeout details.
**Current**: `reqwest` default timeout applies (no explicit timeout set in most providers).
**TODO**: Add configurable timeout to provider builders.

### EC-NET-02: DNS resolution failure
**Trigger**: Host unreachable (Ollama not running, wrong base URL).
**Expected**: `LlmError::NetworkError`.
**Status**: Propagated from `reqwest`.

### EC-NET-03: TLS certificate error
**Trigger**: Self-signed cert in local deployment.
**Expected**: `LlmError::NetworkError` with TLS details.
**Note**: `reqwest` enforces TLS verification by default. Operators can disable it
         with `EDGEQUAKE_TLS_SKIP_VERIFY=true` (not yet implemented; would be insecure).

### EC-NET-04: HTTP 429 Rate limit
**Trigger**: Too many requests per minute.
**Expected**: `LlmError::RateLimitError` (currently `LlmError::ApiError`).
**TODO**: Parse `Retry-After` header and return typed rate-limit error.

---

## Streaming edge cases

### EC-STREAM-01: SSE stream ends mid-token
**Trigger**: Network interruption during streaming chat response.
**Expected**: `LlmError::NetworkError` or `LlmError::StreamError`.
**Current**: Propagated from the SSE parser.

### EC-STREAM-02: Final `data:` line without trailing newline
**Trigger**: Some SSE implementations omit the final `\n\n` before closing.
**Expected**: Final event is still parsed and yielded.
**Status**: Fixed (see user memory: SSE parser must flush buffered data at EOF).

### EC-STREAM-03: `[DONE]` sentinel
**Trigger**: OpenAI-compatible APIs send `data: [DONE]` to signal end of stream.
**Expected**: Stream ends without error after `[DONE]`.
**Status**: Handled in OpenAI-compatible provider.

---

## Mock provider edge cases

### EC-MOCK-01: Mock returns deterministic responses
**Contract**: `MockProvider::chat()` returns a fixed response regardless of input.
**Use**: Unit tests that must not call real APIs.
**Risk**: Tests that rely on mock not accidentally use a real provider if env vars are set.
**Guard**: `MockProvider` is constructed explicitly; env-based auto-detection must not
         return a mock unless explicitly requested (see user memory: explicit mock requests
         should short-circuit env-based provider auto-detection).

---

## Version compatibility edge cases

### EC-COMPAT-01: Default `max_batch_size()` of 2048
**Risk**: New providers added without overriding `max_batch_size()` inherit 2048,
         which may exceed their actual limit.
**Mitigation**: The `preflight` CI job runs `cargo test --locked` which includes
              provider surface tests asserting `max_batch_size()` values.
**Recommendation**: Every new provider MUST add a test asserting `max_batch_size()`.

### EC-COMPAT-02: Cargo.lock drift
**Trigger**: `cargo update` pulls in a dependency with a breaking API change.
**Mitigation**: All CI steps use `--locked` flag. `Cargo.lock` is committed.

---

## Regression test matrix

| Test name | EC IDs covered | Provider |
|---|---|---|
| `test_embed_empty_input` | EC-EMBED-01 | All |
| `test_embedding_dimension` | INV-03 (batch size assertion) | Mistral |
| `test_embedding_response_deserialization` | EC-EMBED-07 | Mistral |
| `test_embed_batched_splits_at_limit` | EC-BATCH-03, EC-BATCH-04 | All |
| `test_reserved_headers_not_overridden` | EC-HEADER-02, INV-06 | Mistral |
| `test_header_crlf_rejected` | SC-HEADER-01 | Mistral |
| `test_from_env_missing_api_key` | EC-AUTH-01 | Mistral |

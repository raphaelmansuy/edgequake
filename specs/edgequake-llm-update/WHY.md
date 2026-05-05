# WHY: edgequake-llm update roadmap

> **Context**: This document captures the root-cause reasoning behind the changes introduced
> in `edgequake-llm` v0.6.16. It exists to prevent future regressions and to guide
> contributors who must understand the *why* before touching these code paths.

---

## 1. WHY does edgequake-llm need its own crate?

EdgeQuake separates LLM provider adapters into a standalone crate for three reasons:

| Reason | Detail |
|---|---|
| **Independent release cadence** | Provider API quirks (rate limits, new models, auth changes) evolve faster than the core RAG pipeline. A standalone crate lets us patch providers without bumping edgequake-core. |
| **Composability** | Any Rust project (not just EdgeQuake) can depend on `edgequake-llm` for uniform `LlmProvider` / `EmbeddingProvider` trait abstractions over OpenAI, Mistral, Anthropic, etc. |
| **Test isolation** | Provider unit tests run without a database or pipeline context, keeping CI fast. |

---

## 2. WHY did the v0.6.15 CI publish fail? (Run #25365481334)

### Root cause

`cargo publish` returned **exit code 101** with:

```
error: crate edgequake-llm@0.6.15 already exists on crates.io index
```

### How it happened

1. The `max_batch_size()` Mistral fix was developed locally on branch `fix/mistral-embed-max-batch-size`.
2. PR #74 was **squash-merged** into `main`. The squash created commit `137754f`.
3. Tag `v0.6.15` was pushed **and** `cargo publish` was run **locally** before CI could run.
4. When the `v0.6.15` tag push triggered the `publish.yml` workflow, crates.io already had the version → CI failed.

### Impact

- The crate itself is correct and functional on crates.io (0.6.15 was published successfully).
- Only the CI record shows a red failure; no user impact.
- No retry is possible because crates.io is immutable.

### Fix (applied in v0.6.16)

The `publish` step in `publish.yml` now checks whether the version already exists
on crates.io and **exits 0 (success)** if so, rather than propagating cargo's exit 101.

See: [CI_PUBLISH_IDEMPOTENCY.md](./CI_PUBLISH_IDEMPOTENCY.md)

---

## 3. WHY does Mistral's embedding API limit matter?

### The hard limit

Mistral's `/v1/embeddings` endpoint enforces **512 inputs per request** at the API gateway
level. Exceeding this returns:

```json
{ "code": "3210", "message": "Too many inputs in request, split into more batches." }
```

This is a **hard transport-layer limit**, not a model-level context window limit.

### Why the default was wrong

The `EmbeddingProvider` trait defaults `max_batch_size()` to **2048** — a reasonable
fallback for most OpenAI-compatible APIs. Mistral never overrode this, so any caller
that relied on `max_batch_size()` for batching (as `embed_with_token_budget` does in
`edgequake-pipeline`) would send batches of up to 2048 items → immediate HTTP 400.

### Why 512 in particular

Empirically confirmed: sending exactly 513 inputs to Mistral returns error 3210.
The Mistral documentation (as of 2026-05) does not state this limit explicitly, which
is why it must be encoded in the provider source and documented here.

### Why the fix belongs in the provider

The `SafetyLimitedEmbeddingProviderWrapper` in `edgequake-api` applies a cap of 512
as an **operator safety rail**, but that cap only works if the safety wrapper is
used. Embedding the limit in `MistralProvider::max_batch_size()` ensures it is
correct **unconditionally** — even if the safety wrapper is absent, bypassed, or
the library is used outside EdgeQuake.

See: [BATCH_SIZE_LIMITS.md](./BATCH_SIZE_LIMITS.md)

---

## 4. WHY does header propagation matter? (Issue #132)

### The B2B multi-tenant problem

EdgeQuake is used as a platform-in-a-platform: a B2B provider runs EdgeQuake on behalf
of end-customers. Their internal services communicate via **standardised HTTP headers**:

- `x-request-id` / `x-correlation-id` — distributed tracing
- `x-tenant-id` — cost allocation and audit trails
- `x-api-version` — contract versioning
- Custom HMAC / security tokens

When EdgeQuake forwards requests to the LLM provider (Mistral, OpenAI, etc.), it
currently strips all custom headers. This breaks:

1. **Distributed tracing**: the LLM call does not appear in the tenant's trace tree.
2. **Cost attribution**: LLM providers that support `x-tenant-id` routing cannot route.
3. **Audit compliance**: the LLM vendor's audit log shows EdgeQuake's server IP / key,
   not the originating tenant context.

### Why pass-through in the library (not just in edgequake-api)

If header forwarding is implemented only in `edgequake-api` via a middleware wrapper,
every future provider must independently remember to forward headers. Encoding it in
`edgequake-llm` at the provider level means:
- New providers get header forwarding for free via the builder pattern.
- The contract (`with_extra_headers`) is visible in public API docs on docs.rs.
- Tests in the library can verify forwarding without requiring a full server.

See: [HEADER_PROPAGATION.md](./HEADER_PROPAGATION.md)

---

## 5. Cross-reference map

| Spec | What it covers | Related code |
|---|---|---|
| [WHY.md](./WHY.md) | Rationale for all changes | — |
| [BATCH_SIZE_LIMITS.md](./BATCH_SIZE_LIMITS.md) | Per-provider input limits, error codes, test coverage | `src/providers/mistral.rs` |
| [HEADER_PROPAGATION.md](./HEADER_PROPAGATION.md) | Design + edge cases for `with_extra_headers` | `src/providers/*.rs` |
| [CI_PUBLISH_IDEMPOTENCY.md](./CI_PUBLISH_IDEMPOTENCY.md) | CI fix: skip publish if already exists | `.github/workflows/publish.yml` |
| [EDGE_CASES.md](./EDGE_CASES.md) | Exhaustive edge-case registry | all providers |
| [RELEASE_CHECKLIST.md](./RELEASE_CHECKLIST.md) | Step-by-step checklist for future releases | — |

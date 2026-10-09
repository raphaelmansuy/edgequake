---
title: "LLM cache scope decision"
description: "Decision record GAP-091-14: the llm_cache table is keyed by content hash and namespace, with no tenant or workspace column, so workspaces in one namespace share cache entries. Explains why and the accepted risks."
---

# LLM cache scope decision (SPEC-091, GAP-091-14)

**Status:** Accepted on 2026-07-30 (IW0). A contract test pins it: `edgequake-storage/tests/contract_spec091_llm_cache_scope.rs`.

The LLM cache stores the answers that a language model gave to a prompt, so the same prompt does not cost money twice. This record explains who can share an entry.

## Decision

The table `public.llm_cache` is keyed by a content hash inside a storage namespace. The primary key is `(cache_key, namespace)` (migration 124). The table has no `tenant_id` or `workspace_id` column, on purpose.

- Two workspaces in the same namespace share entries. A hit returns the output that was computed earlier for the same prompt and model.
- Different namespaces never see each other. Every read and write in `adapters/postgres/llm_cache.rs` filters on the namespace.

## Reason

The cache guards against recomputing. It is not document data. The same input gives the same output, so sharing across workspaces is safe. It also saves money when workspaces ingest overlapping documents or send identical keyword prompts. A lost entry costs one recomputation and never harms correctness. The table comment says the same: rows are recomputable and may expire.

## Accepted risks

- **Timing side channel.** Inside one namespace, workspace B can notice from latency that workspace A already sent the same prompt. No content crosses, because the output depends only on the input, but the access pattern leaks. This is accepted: a namespace maps to a deployment trust boundary.
- **Provider drift.** An entry written for model X is served to a workspace set up for model Y only if the cache keys match. The key includes the prompt hash, and multimodal keys include `{mode}-{type}`. For extraction caches, the model identity is part of the hashed prompt. If you need hard isolation per tenant, deploy one namespace per tenant. The namespace is already a configuration setting.

## Consequences

- Do not add a workspace or tenant column to `llm_cache` without updating this record and the contract test.
- Deleting a document invalidates cache entries by namespace and key. No per-workspace sweep exists, and none is needed.
- `llm_cache` is also the durable layer of the keyword cache (SPEC-103). See [jsonb-envelope-acceptance.md](./jsonb-envelope-acceptance.md) for why its `value` column stays JSONB.

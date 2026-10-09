---
title: Workspace model roles
description: What each model role does, where to configure it, and which roles can use a saved Connection.
---

A role is a job that needs a model. A workspace can give each role its own provider and model, and can point some roles at a saved Connection. Roles you leave empty inherit the workspace default, then the server default.

| Role | Job | Configured by | Uses a Connection? |
|------|-----|---------------|--------------------|
| `extract` | Pull entities and relations from documents | Workspace `llm_*`, `llm_roles.extract`; env `EDGEQUAKE_EXTRACT_LLM_PROVIDER`, `EDGEQUAKE_EXTRACT_LLM_MODEL` | Yes |
| `query` | Write the final answer | Request override, `llm_roles.query` | Yes |
| `keyword` | Pull search keywords from a question | `EDGEQUAKE_KEYWORD_LLM_PROVIDER` and `EDGEQUAKE_KEYWORD_LLM_MODEL`, then `llm_roles.keyword` | No |
| `summary` | Merge entity descriptions | `llm_roles.summary` | No |
| `embedding` | Turn text into vectors | Workspace `embedding_*`; env `EDGEQUAKE_EMBEDDING_PROVIDER`, `EDGEQUAKE_EMBEDDING_MODEL` | No |
| `vlm` (vision) | Read PDF pages as images | Upload or workspace `vision_llm_*`, or `llm_roles.vlm` | No |
| `reranker` | Re-order retrieved chunks | `EDGEQUAKE_RERANKER_PROVIDER`, `EDGEQUAKE_RERANKER_MODEL`, `EDGEQUAKE_RERANKER_BASE_URL` (whole server) | No |
| `decision` | Decision-style extraction (SPEC-160) | `EDGEQUAKE_DECISION_*` plus workspace metadata | No |

The core `llm_roles` map holds five roles: `extract`, `query`, `summary`, `vlm` and `keyword`. Embedding, reranker and decision are configured through their own settings. The Settings page shows all eight in one table.

## Example

Workspace metadata:

```json
{
  "llm_roles": {
    "extract": { "provider": "openai", "model": "gpt-5.4-mini" },
    "query": {
      "provider": "openai-compatible",
      "model": "my-model",
      "connection_id": "11111111-1111-1111-1111-111111111111"
    }
  }
}
```

## Fallback is silent

If a role's `connection_id` is not a valid UUID, the row does not exist, PostgreSQL is not available, or the client cannot be built, EdgeQuake does not return an error. It uses the next choice instead. A key that cannot be decrypted (for example after changing `EDGEQUAKE_SECRETS_KEY`) is treated as no key. If answers come from a different model than you expect, check the Connection with **Test connection** and check `/health`.

## Change embeddings with care

Vectors from two embedding models are not comparable. After you change the embedding provider, model or dimension, rebuild the stored embeddings. See [Embedding backfill](../operations/embedding-registry-backfill.md).

Related: [Providers overview](index.md), [Provider security](security.md).

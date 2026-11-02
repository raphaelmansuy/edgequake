---
title: "Upgrade to EdgeQuake v0.39.0"
description: "Upgrade from v0.38.4 to v0.39.0. Schema moves 224 → 226."
---

# Upgrade to EdgeQuake v0.39.0

> **From:** v0.38.4 (schema **224**) · **To:** v0.39.0 (schema **226**)

This is the first cut since v0.38.0 that applies new migrations. Run `edgequake migrate` **before** you start the new API replicas.

Two migrations land:

| Version | File | Adds |
| ------- | ---- | ---- |
| **225** | `225_spec180_normal_rag.sql` | The Normal RAG profile columns (expand) |
| **226** | `226_spec181_admin.sql` | `subscription_plans`, real quota columns, the archive columns, `scope_purge_receipts`, and two guard triggers |

Both are `expand`-phase and additive. A v0.38.4 binary keeps working against the migrated schema, so the migration can run ahead of the image bump.

## Order of operations

```bash
# 1. Back up, then migrate. The advisory lock makes this safe with the old
#    replicas still serving.
edgequake migrate

# 2. Roll the API replicas to 0.39.0.
# 3. Roll the WebUI.
```

`EDGEQUAKE_SCHEMA_GATE` controls what a replica does when it boots ahead of the migration. The default `fail` exits with code 78. Helm and compose set `wait`, which serves `/live` and answers `503` on `/ready` until the ledger reports the schema current.

## What changes for an operator

**Normal RAG workspaces (SPEC-180).** A workspace can now be created as `normal`: chunks, dense embeddings and BM25, with no knowledge graph and no chat model at ingest. The choice is permanent at create time. On a normal workspace the graph endpoints answer `409` and the graph-only retrieval modes answer `422`. Nothing changes for an existing graph workspace.

**Admin consolidation (SPEC-181).** The eight-step create wizards are replaced by two three-decision dialogs, one for an organization and one for a workspace. Platform Admin gains a capability catalog with a single `GET /me/access` answer, a grant editor, and archive/restore. Navigation is gated by the same answer, so a capability an operator does not hold is not rendered.

Three defects are fixed by this cut, and each one matters to an operator who already has data:

- Creating an organization now grants the creator an owner membership. Before, the row existed with no member, so nobody could administer it.
- Deleting an organization now purges all nine storage planes. Before, three were purged and six were orphaned.
- Quota has one source of truth. Before, four different columns disagreed, and `trg_workspace_quota_guard` now enforces the plan limit on write.

`trg_org_keeps_an_owner` refuses the last owner removal from an organization. If your automation revokes memberships, expect that error instead of a silent lockout.

**Provider selection in development (`make dev`).** `make dev` used to export `OLLAMA_HOST` unconditionally. `ProviderFactory::from_env` reads `OLLAMA_HOST` before any cloud key, so an exported `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GEMINI_API_KEY` or `XAI_API_KEY` was ignored and the stack ran on Ollama. The Makefile now resolves the provider through `scripts/dev_provider_detect.sh` and exports `EDGEQUAKE_LLM_PROVIDER` / `EDGEQUAKE_LLM_MODEL` explicitly. This affects the local stack only; a deployed container reads the same environment variables as before.

## Dependency note

`edgequake-llm` moves to **0.10.10**, which adds Vertex AI authentication through Application Default Credentials. `edgequake-litellm` stays at 0.10.4. No `GOOGLE_APPLICATION_CREDENTIALS` file is required; the client accepts the active `gcloud` account as well.

## Frontend build note

`next` moves to 16.4.3 and `dompurify` to 3.4.17. A KaTeX sink is closed: `trust: true` is gone from the render options, so `\href{javascript:...}` no longer emits a live link. If you render untrusted markdown, rebuild the image rather than reusing a 0.38.x frontend bundle.

**SPEC-001 Acc:** not re-scored on this cut. Mix retrieval is unchanged by SPEC-180, which adds a separate workspace profile rather than altering mix behaviour. The latest valid pack remains [`medical-mid-20261024T031725Z`](../../specs/001-benchmark/e2e/artifacts/history/medical-mid-20261024T031725Z/) (`valid: true`). EdgeQuake **0.8084**, LightRAG **0.7681**, delta **+0.0403**, 95% CI **[-0.0241, +0.1036]** (n_paired 72). The interval includes 0, so this is a statistical tie. EdgeQuake does not claim a win.

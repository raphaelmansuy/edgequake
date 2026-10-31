# 05 — Profile law

Parent: [README](README.md) · Prev: [04-code-findings](04-code-findings.md) · Next: [06-data-and-db-contract](06-data-and-db-contract.md)

## Wire

| Value | Meaning |
|-------|---------|
| `graph` | Hybrid GraphRAG. Default. [LAW-180-12](01-first-principles.md). |
| `normal` | Chunks, dense embeddings, BM25. No graph. |

Parse is case-insensitive. Unknown strings, including `none`, `chunk_only`, and empty, are **400** `invalid_rag_profile` on create. They are not inherit, and they are not `graph`. The fleet env does not fill this field ([F-180-08](04-code-findings.md), [F-180-01](04-code-findings.md)).

Omitted on create means `graph`.

## Capability matrix

`ProfileCapabilities::from(profile)` is the only mapping.

| Capability | `graph` | `normal` |
|------------|---------|----------|
| `extracts_graph` | true | false |
| `runs_gleaning` | from workspace gleaning settings | false |
| `runs_summary_llm` | true | false |
| `merges_graph` | true | false |
| `refreshes_communities` | true | false |
| `embeds_entities` | true | false |
| `embeds_relationships` | true | false |
| `embeds_chunks` | true | true |
| `embeds_images` | false in this pack (graph keeps caption-as-text) | true only when workspace modalities include `image` |
| `writes_bm25` | false (keeps `ts_rank_cd`) | true |
| `queries_graph` | true | false |
| `default_query` | Mix | `NormalRetriever` |
| `allows_graph_modes` | true | false |
| `shows_graph_ui` | true | false |
| `judges_extract_usage` | true | false |
| `judges_summary_usage` | true | false |
| `zero_entities_is_failure` | true | false |
| `skips_graph_on_delete` | false | true |

`embeds_images` is not a property of `normal` alone. It is `normal && modalities contains image`. Graph workspaces do not gain image vectors in this pack, so a graph workspace that wants image embeddings is a non-goal. The operator creates a normal workspace.

## Precedence

```text
create body rag_profile
    → column workspaces.rag_profile
        → ProfileCapabilities
            → ingest flags, query entry, UI, MCP

document extraction_mode     ignored and rejected on normal (LAW-180-13)
EDGEQUAKE_INGEST_PROFILE     ignored when the column is set (always, after migration 225)
EDGEQUAKE_EXTRACTION_MODE    applies only when extracts_graph
```

Embedding model precedence stays SPEC-178: request, then workspace, then tenant, then env. On create of a normal workspace the chosen embedding model is stored as it is today. Modalities (`text` or `text,image`) are a new create field, default `text`, immutable like the profile. Changing the embedding model still goes through rebuild-embeddings ([LAW-180-5](01-first-principles.md)). Changing modalities is **409** `embedding_modalities_immutable`. A model change to a text-only model while modalities are `text,image` is **422** `embedding_model_not_multimodal` on the rebuild request.

## What normal does with SPEC-160

Extraction mode is a strategy for building a graph. A normal workspace has no graph, so:

- Create may omit `extraction_mode`. If the body sets it to anything but inherit-empty, the API returns **400** `extraction_mode_not_applicable`.
- The settings card for extraction mode is hidden.
- Decision backend (Tev1) is not constructed and not health-checked for that workspace.

This avoids a third enum variant and the `none` inherit clash ([F-180-04](04-code-findings.md), [F-180-08](04-code-findings.md)).

## Reprocess, delete, rebuild

| Action | Normal behavior |
|--------|-----------------|
| Reprocess document | Re-chunk, re-embed, rebuild BM25 postings for that document. No snapshot of extractions. `ReprocessMode::EntitiesOnly` and `MergeOnly` return **409** `reprocess_mode_not_applicable`. `Full` is the only mode. |
| Delete document | Tombstone, delete chunk rows, chunk embeddings, BM25 postings, KV, originals. No graph cascade. |
| Rebuild embeddings | Existing handler. Clears vectors, re-queues documents. Also rebuilds BM25 if the analyzer would change (it does not, unless FTS config changes — FTS config is immutable too). |
| Rebuild knowledge graph | **409** `profile_has_no_graph`. |

## Fingerprint

`process_fingerprint` gains a `rp=` segment (`graph` or `normal`) so a checkpoint written under another shape cannot be reused. Because the profile cannot change, this matters for retries that loaded a pre-spec snapshot and for tests. Stale graph snapshots are not applied when `extracts_graph` is false.

## In-memory twin

[workspace_service/in_memory.rs](../../edgequake/crates/edgequake-core/src/workspace_service/in_memory.rs) stores the same column in the struct. Tests that build `CreateWorkspaceRequest` with `..Default::default()` get `graph`. A test that wants normal sets the field. Both the Postgres and the memory service reject updates.

## Module home

| Piece | Crate | Why |
|-------|-------|-----|
| `RagProfile`, `ProfileCapabilities` | `edgequake-pipeline` or `edgequake-core` | Pure. No Axum, no SQL. Prefer `edgequake-core` if the query crate must not depend on pipeline internals. **Chosen home: `edgequake-core`**, next to workspace types, so query, pipeline, and API all depend downward. |
| `apply` onto `PipelineConfig` | `edgequake-pipeline` | Calls the existing `IngestProfile::apply_to` when capabilities say so, plus the new gates. |
| HTTP DTO | `edgequake-api` | Flattened fragment, same shape as extraction mode, but not the same type. |
| UI constants | `edgequake_webui/src/constants/rag-profile.ts` | One list, used by the wizard, the card, and the mode menu. |

Query crate depends on `edgequake-core` already. It must not re-parse the wire string. It receives `ProfileCapabilities`.

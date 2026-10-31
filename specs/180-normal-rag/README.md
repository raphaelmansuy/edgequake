# SPEC-180 — Normal RAG workspace

> **Status:** Specification. No code in this pack.
> **Product pin:** EdgeQuake v0.38.3. Migration file head before this pack is
> [224_spec179_provider_refresh.sql](../../edgequake/migrations/224_spec179_provider_refresh.sql).
> This pack reserves migration **225**.
> **Scope:** A workspace created as **Normal RAG** stores chunks, dense embeddings,
> and a BM25 lexical index. It does not build a knowledge graph. The same workspace
> can embed text and images in one vector space when the embedding model is
> multimodal. Search can be scoped by document kind.
> **Inherits:** [SPEC-160](../160-tev1/README.md) extraction mode resolver ·
> [SPEC-091](../091-simplify-data-layer/README.md) typed embeddings ·
> [SPEC-046](../046-graphrag-study/00-INDEX.md) hybrid retrieval ·
> [SPEC-174](../174-document-security/README.md) clearance ·
> [SPEC-178](../178-provider-and-model-choice/README.md) model choice ·
> [SPEC-047](../047-rag-evaluation/000-index.md) `IngestProfile::ChunkOnly` (fleet env only).
> **Does not replace:** Graph workspaces. `rag_profile = graph` is the default and
> keeps today's ingest and query behavior ([LAW-180-12](01-first-principles.md)).

## Words

| Word | Meaning in this pack |
|------|----------------------|
| Profile | `rag_profile`: `graph` or `normal`. Set once, at workspace creation. |
| Normal RAG | Chunks + dense vectors + BM25. No entities, relations, communities, or graph query. |
| Graph RAG | Today's Hybrid GraphRAG. Unchanged. |
| Capability | A boolean on `ProfileCapabilities`. Callers ask the struct. They do not compare the enum in ten places. |
| Source modality | What a chunk was embedded from: `text` or `image`. Not the chart/figure/table label from SPEC-134. |
| Document kind | Family of the source file: `pdf`, `image`, `markdown`, `text`, `office`. |
| Keyword | A BM25 term. Not a separate LLM keyword-extraction stage. |

## Start here

1. [00-why.md](00-why.md) — Five WHYs
2. [01-first-principles.md](01-first-principles.md) — LAW-180-1…16
3. [02-research.md](02-research.md) — Papers and vendor docs tied to decisions
4. [03-product-spec.md](03-product-spec.md) — Personas, jobs, cost, acceptance
5. [04-code-findings.md](04-code-findings.md) — F-180-* with file and line
6. [05-profile-law.md](05-profile-law.md) — Capability matrix and precedence
7. [06-data-and-db-contract.md](06-data-and-db-contract.md) — Migration 225
8. [07-ingest-pipeline.md](07-ingest-pipeline.md) — What the worker skips
9. [08-multimodal-embedding.md](08-multimodal-embedding.md) — `gemini-embedding-2` on Vertex
10. [09-retrieval-and-fusion.md](09-retrieval-and-fusion.md) — BM25, RRF, rerank, scope
11. [10-api-contract.md](10-api-contract.md) — Routes and error codes
12. [11-ux-ui-spec.md](11-ux-ui-spec.md) — Copy and states
13. [12-screens.md](12-screens.md) — ASCII wireframes
14. [13-frontend-architecture.md](13-frontend-architecture.md) — Wizard, cards, composer
15. [14-edge-cases.md](14-edge-cases.md) — EC-180 register
16. [15-implementation-plan.md](15-implementation-plan.md) — Waves W0–W6
17. [16-e2e-test-matrix.md](16-e2e-test-matrix.md) — One test per EC
18. [17-cross-ref.md](17-cross-ref.md) — WHY, law, finding, EC, wave, test, lens
19. Lenses → [`lenses/`](lenses/)

## Locked decisions

1. **The profile is the shape of the data.** It is chosen at creation and cannot change. [LAW-180-1](01-first-principles.md).
2. **One capability struct.** Ingest, query, delete, MCP, and UI read `ProfileCapabilities`. [LAW-180-2](01-first-principles.md).
3. **Normal ingest calls no chat LLM.** Extraction, gleaning, summary, and community reports do not run. [LAW-180-3](01-first-principles.md).
4. **Embeddings fail closed.** A missing Vertex credential does not become the mock embedder. [LAW-180-4](01-first-principles.md).
5. **One vector space per workspace.** Model, dimension, and modalities are fixed together. A model change uses the existing rebuild. [LAW-180-5](01-first-principles.md).
6. **Fuse ranks, not raw scores.** RRF with k = 60. `min_score` does not filter RRF scores. [LAW-180-6](01-first-principles.md).
7. **The lexical leg is BM25.** Default path needs no Postgres extension. [LAW-180-7](01-first-principles.md).
8. **Graph query modes are refused** on a normal workspace. An omitted mode uses normal retrieval. [LAW-180-8](01-first-principles.md).
9. **Text and image share one space** when the model embeds both. `source_modality` is a label. [LAW-180-9](01-first-principles.md).
10. **Document-kind scope is a filter,** intersected with clearance. It is not a second index. [LAW-180-10](01-first-principles.md).
11. **Graph surfaces are absent,** not broken, on a normal workspace. [LAW-180-11](01-first-principles.md).
12. **Existing workspaces stay graph.** The column default is `graph`. [LAW-180-12](01-first-principles.md).
13. **No per-document profile override.** [LAW-180-13](01-first-principles.md).
14. **Zero entities is success** when the profile does not extract. [LAW-180-14](01-first-principles.md).
15. **Every EC-180 has a named test.** [LAW-180-15](01-first-principles.md).
16. **Keywords are BM25 terms,** not an ingest-time LLM pass. [LAW-180-16](01-first-principles.md).

## Non-goals

- Late-interaction multi-vector retrieval (ColPali). Recorded in [02-research.md](02-research.md).
- Switching a workspace from `graph` to `normal` or the reverse. Create a new workspace.
- A tenant-wide default profile. Each workspace chooses.
- Audio and video embeddings. The model supports them. This pack embeds text and still images.
- Replacing `tsvector` on graph workspaces. Graph workspaces keep today's FTS until a later spec.
- Publishing a new `edgequake-llm` crate. The multimodal client lives in this repo. Upstream is a follow-up.
- True BM25 via `pg_search` as a requirement. It is an optional backend when the extension is already installed.

## Job in one picture

```text
  Create workspace
  +---------------------------+
  | rag_profile = graph       |----> today's pipeline (unchanged)
  | rag_profile = normal      |
  +-------------+-------------+
                |
                v
  +---------------------------+
  | ProfileCapabilities       |  one struct, every gate
  +--+----------+----------+--+
     |          |          |
     v          v          v
  Ingest     Query       UI / MCP
  chunk      dense+BM25  hide graph
  embed      RRF+rerank  kind chips
  no LLM     no graph    image query
  extract
```

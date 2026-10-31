# 16 — E2E and test matrix

Parent: [README](README.md) · Prev: [15-implementation-plan](15-implementation-plan.md) · Next: [17-cross-ref](17-cross-ref.md)

Layers: **unit** (no Postgres), **contract** (API codes, schema), **pg** (Postgres integration), **e2e** (worker + query, fake LLM), **live** (`#[ignore]`, Vertex project `mobioos-506015`), **ui** (unit or Playwright).

Hermetic e2e uses `edgequake-fake-llm` for chat and, from W4, for `/embedContent`. A test that counts chat completions fails if the count is not zero on normal ingest (`T-180-ingest-zero-chat`).

| Test | Layer | Wave | ECs |
|------|-------|------|-----|
| `T-180-create-default-graph` | contract | W1 | EC-180-01, EC-180-07 |
| `T-180-create-invalid-profile` | contract | W1 | EC-180-03 |
| `T-180-update-immutable` | contract | W1 | EC-180-04 |
| `T-180-update-name-ok` | contract | W1 | EC-180-05 |
| `T-180-slug-conflict` | contract | W1 | EC-180-06 |
| `T-180-env-ignored` | e2e | W1 | EC-180-08 |
| `T-180-modalities-graph` | contract | W1 | EC-180-56 |
| `T-180-trigger` | pg | W1 | EC-180-58 |
| `T-180-memory-immutable` | unit | W1 | EC-180-59 |
| `T-180-hnsw-3072` | pg | W1 | EC-180-32 |
| `T-180-ingest-zero-chat` | e2e | W2 | EC-180-02 |
| `T-180-upload-mode-rejected` | contract | W2 | EC-180-09 |
| `T-180-reprocess-mode` | contract | W2 | EC-180-10 |
| `T-180-delete-no-graph` | e2e | W2 | EC-180-11 |
| `T-180-no-snapshot-replay` | e2e | W2 | EC-180-12 |
| `T-180-completed-zero-entities` | e2e | W2 | EC-180-13 |
| `T-180-progress-copy` | contract | W2 | EC-180-14 |
| `T-180-zero-chunks` | e2e | W2 | EC-180-15, EC-180-16 |
| `T-180-scanned-pdf` | e2e | W2/W4 | EC-180-17 |
| `T-180-dedup` | e2e | W2 | EC-180-47 |
| `T-180-no-community` | e2e | W2 | EC-180-48 |
| `T-180-no-summary-judge` | e2e | W2 | EC-180-49 |
| `T-180-cost-fields` | e2e | W2 | EC-180-50 |
| `T-180-df-concurrent` | pg | W2 | EC-180-51 |
| `T-180-df-delete` | pg | W2 | EC-180-52 |
| `T-180-stopwords` | unit | W3 | EC-180-33 |
| `T-180-or-query` | pg | W3 | EC-180-34 |
| `T-180-fts-config` | contract | W3 | EC-180-35 |
| `T-180-rrf-keeps-hits` | unit | W3 | EC-180-36 |
| `T-180-dense-only` | unit | W3 | EC-180-37 |
| `T-180-mode-mix` | contract | W3 | EC-180-38 |
| `T-180-mode-default` | e2e | W3 | EC-180-39 |
| `T-180-kind-filter` | pg | W3 | EC-180-40 |
| `T-180-kind-and-clearance` | pg | W3 | EC-180-41 |
| `T-180-rls-terms` | pg | W3 | EC-180-42 |
| `T-180-rerank-image` | unit | W3 | EC-180-55 |
| `T-180-image-on-text-ws` | contract | W4 | EC-180-18 |
| `T-180-image-chunk` | e2e | W4 | EC-180-19 |
| `T-180-text-to-image` | live | W4 | EC-180-20 |
| `T-180-image-to-image` | live | W4 | EC-180-21 |
| `T-180-image-query-rejected` | contract | W4 | EC-180-22 |
| `T-180-six-image-cap` | unit | W4 | EC-180-23 |
| `T-180-image-bytes` | unit | W4 | EC-180-24 |
| `T-180-image-corrupt` | contract | W4 | EC-180-25 |
| `T-180-image-gif` | contract | W4 | EC-180-26 |
| `T-180-token-budget` | unit | W4 | EC-180-27 |
| `T-180-no-mock` | unit | W4 | EC-180-28 |
| `T-180-token-refresh` | unit | W4 | EC-180-29 |
| `T-180-embed-429` | unit | W4 | EC-180-30 |
| `T-180-dim-check` | unit | W4 | EC-180-31 |
| `T-180-rebuild-model` | contract | W4 | EC-180-53 |
| `T-180-query-both` | e2e | W4 | EC-180-54 |
| `T-180-answer-caption` | unit | W4 | EC-180-60 |
| `T-180-mcp-search` | contract | W5 | EC-180-43 |
| `T-180-mcp-entity` | contract | W5 | EC-180-44 |
| `T-180-ui-knowledge` | ui | W5 | EC-180-45 |
| `T-180-ui-wizard` | ui | W5 | EC-180-46 |
| `T-180-ui-query` | ui | W5 | EC-180-40 (chips), composer |
| `T-180-ui-switch` | ui | W5 | EC-180-57 |

`T-180-text-to-image` and `T-180-image-to-image` also run against the fake embedder as e2e with a fixed vector so CI is red/green without Google. The live pair is an extra `#[ignore]` test with the same name suffix `_live`. The EC is not done until the live test has been run once on `mobioos-506015` and the spike note records the date. CI does not call Vertex.

## Live procedure

```text
gcp-use-mobioos
export GOOGLE_CLOUD_PROJECT=mobioos-506015
export GOOGLE_ACCESS_TOKEN="$(gcloud auth print-access-token)"
cargo test -p edgequake-api --test e2e_spec180_vertex -- --ignored
```

The test process must set the project itself as well, so a dirty environment cannot point at another project. Assert: text query returns the image document in the top 5; image query returns that image above a unrelated text chunk; `doc_kinds=["pdf"]` excludes the image.

## Graph regression

These existing tests stay required on W3 and W6. They are not renamed.

- `edgequake-query` `contract_bm25_retrieval`, `e2e_spec046_hybrid_rag`
- `edgequake-api` SPEC-160 extraction admission tests
- `edgequake-storage` `contract_spec091_chunk_fts`

A normal-profile test that fails one of these is a [LAW-180-12](01-first-principles.md) break.

## Eval (report, not a gate)

`spikes/eval.md` after W4: one corpus, two workspaces, same embedding model.

| Measure | Graph | Normal |
|---------|-------|--------|
| Chat LLM calls at ingest | recorded | 0 |
| Embed calls | recorded | recorded |
| Recall@10 on the labeled set | recorded | recorded |
| Median query latency | recorded | recorded |

If normal recall is far below graph on multi-hop items, the note says so and points at R-180-1. It does not block the wave.

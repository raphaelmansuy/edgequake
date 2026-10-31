# 14 — Edge cases

Parent: [README](README.md) · Prev: [13-frontend-architecture](13-frontend-architecture.md) · Next: [15-implementation-plan](15-implementation-plan.md)

Every row has a mitigation and a test name. The matrix in [16-e2e-test-matrix](16-e2e-test-matrix.md) maps the name to a file. [LAW-180-15](01-first-principles.md).

| ID | Case | Risk | Mitigation | Test |
|----|------|------|------------|------|
| EC-180-01 | Create omits `rag_profile` | Silent normal would change the product | Default `graph` | `T-180-create-default-graph` |
| EC-180-02 | Create `normal` | Flags forgotten, graph still built | Capabilities drive every gate | `T-180-ingest-zero-chat` |
| EC-180-03 | Create `rag_profile=none` or `chunk_only` | Inherit or silent Full ([F-180-08](04-code-findings.md)) | 400 `invalid_rag_profile` | `T-180-create-invalid-profile` |
| EC-180-04 | PUT sends `rag_profile` even if unchanged | Client "save" flips or 500s from the trigger | 409 before write; SDK strips the field | `T-180-update-immutable` |
| EC-180-05 | PUT changes the display name on normal | Immutability too broad | Name update 200; column unchanged | `T-180-update-name-ok` |
| EC-180-06 | Two creates, same slug | Profile check masks the slug error | Existing slug conflict wins; profile not half-written | `T-180-slug-conflict` |
| EC-180-07 | Old OpenAPI client | Unknown field ignored, or rejected | New fields optional; omit means graph | `T-180-create-default-graph` |
| EC-180-08 | Fleet `EDGEQUAKE_INGEST_PROFILE=chunk_only` | Graph workspaces lose extraction | Column wins; env ignored | `T-180-env-ignored` |
| EC-180-09 | Upload `extraction_mode=llm` on normal | Per-doc graph inside a normal workspace | 400 `extraction_mode_not_applicable` | `T-180-upload-mode-rejected` |
| EC-180-10 | Reprocess `entities_only` | Worker looks for a graph | 409 `reprocess_mode_not_applicable` | `T-180-reprocess-mode` |
| EC-180-11 | Delete | Graph cascade errors on empty graph | Skip cascade; chunks and vectors gone | `T-180-delete-no-graph` |
| EC-180-12 | Retry loads an old extraction snapshot | Graph rows appear | Ignore snapshot when `extracts_graph` is false | `T-180-no-snapshot-replay` |
| EC-180-13 | Success with zero entities | `partial_failure` ([F-180-02](04-code-findings.md)) | Status `completed` | `T-180-completed-zero-entities` |
| EC-180-14 | Progress copy | UI says "knowledge graph" | Profile-aware messages | `T-180-progress-copy` |
| EC-180-15 | Empty text file | Completed with no evidence | `failed` `no_chunks` | `T-180-zero-chunks` |
| EC-180-16 | Whitespace-only document | Same as empty | `no_chunks` | `T-180-zero-chunks` |
| EC-180-17 | Scanned PDF, vision off, no text | Empty index | `no_chunks` if no page image is embedded; if images are on and a raster exists, image chunks still index | `T-180-scanned-pdf` |
| EC-180-18 | PNG upload, modalities `text` | Caption-only surprise | 422 `embedding_model_not_multimodal` | `T-180-image-on-text-ws` |
| EC-180-19 | PNG upload, modalities `text,image` | No vector, or a text-only vector | One image chunk and one embedding | `T-180-image-chunk` |
| EC-180-20 | Text query should find an image | Caption-only retrieval | Dense hit on the image vector | `T-180-text-to-image` |
| EC-180-21 | Image query finds the same image | Endpoint missing | Multipart query image | `T-180-image-to-image` |
| EC-180-22 | Image query on a text workspace | Server embeds with the wrong model | 422 `image_query_not_supported` | `T-180-image-query-rejected` |
| EC-180-23 | More than 6 images in one embed input | Vendor 400 | Chunk builder never emits more than one image per input | `T-180-six-image-cap` |
| EC-180-24 | Image larger than the byte budget | Vendor 400 or OOM | Re-encode JPEG, else `image_too_large` | `T-180-image-bytes` |
| EC-180-25 | Corrupt image | Worker panic | `image_decode_failed` | `T-180-image-corrupt` |
| EC-180-26 | Animated GIF | Silent first frame | `image_animated_unsupported` | `T-180-image-gif` |
| EC-180-27 | Chunk over 8192 tokens | Silent vendor truncation | Split or reject before HTTP | `T-180-token-budget` |
| EC-180-28 | No Vertex credential | Mock vectors that do not match prod | `embedding_auth`, no mock | `T-180-no-mock` |
| EC-180-29 | Token expires mid-batch | Whole document fails | Refresh once and retry the call | `T-180-token-refresh` |
| EC-180-30 | HTTP 429 | Fail the document immediately | Existing backoff, then fail | `T-180-embed-429` |
| EC-180-31 | Vendor returns a different width | Mixed space, bad cosine | `embedding_dimension_mismatch`, no insert | `T-180-dim-check` |
| EC-180-32 | Dimension 3072 | Sequential scan | Partial HNSW from migration 225 | `T-180-hnsw-3072` |
| EC-180-33 | Query is only stopwords | Empty BM25 wipes dense ([F-180-11](04-code-findings.md)) | Skip lexical leg; return dense | `T-180-stopwords` |
| EC-180-34 | Long question, rare term in one chunk | AND `websearch_to_tsquery` misses | OR candidates, BM25 rank | `T-180-or-query` |
| EC-180-35 | French corpus, regconfig `english` | Stemming eats tokens | Create-time `french` or `simple`; immutable | `T-180-fts-config` |
| EC-180-36 | RRF scores under 0.1 | Empty result ([F-180-12](04-code-findings.md)) | Do not apply `min_score` to RRF | `T-180-rrf-keeps-hits` |
| EC-180-37 | BM25 returns nothing | Empty answer | Dense leg alone | `T-180-dense-only` |
| EC-180-38 | `mode=mix` on normal | Graph code runs | 422 `mode_not_supported_by_profile` | `T-180-mode-mix` |
| EC-180-39 | Mode omitted | Mix default ([F-180-09](04-code-findings.md)) | Normal retriever | `T-180-mode-default` |
| EC-180-40 | `doc_kinds=["image"]` | Text chunks leak | SQL predicate | `T-180-kind-filter` |
| EC-180-41 | Kind filter plus clearance | Filter bypasses SPEC-174 | AND with clearance ids | `T-180-kind-and-clearance` |
| EC-180-42 | Two workspaces, shared table | Term stats leak | RLS and `workspace_id` on `chunk_terms` | `T-180-rls-terms` |
| EC-180-43 | MCP `eq_search` | Forces subgraph ([F-180-16](04-code-findings.md)) | `include_subgraph false` | `T-180-mcp-search` |
| EC-180-44 | MCP entity tool | Empty list looks like "no entities yet" | 409 `profile_has_no_graph` | `T-180-mcp-entity` |
| EC-180-45 | User opens `/knowledge` | Broken canvas | Empty state, no nav item | `T-180-ui-knowledge` |
| EC-180-46 | Review step | User misses permanence | Copy in [12-screens](12-screens.md) | `T-180-ui-wizard` |
| EC-180-47 | Duplicate upload | Second graph merge | Existing dedup; no graph write | `T-180-dedup` |
| EC-180-48 | Community refresh | Background graph job | Not scheduled | `T-180-no-community` |
| EC-180-49 | Summary usage gate | Ingest fails closed for a missing summary model | Gate skipped | `T-180-no-summary-judge` |
| EC-180-50 | Cost metadata | Shows extract model ([F-180-06](04-code-findings.md)) | Embedding fields only; `llm_calls=0` | `T-180-cost-fields` |
| EC-180-51 | Concurrent ingests | Lost `df` updates | Workspace row lock around stats | `T-180-df-concurrent` |
| EC-180-52 | Delete then search | Stale `df` | Stats updated in the delete transaction | `T-180-df-delete` |
| EC-180-53 | Rebuild embeddings to a text-only model | Image vectors orphaned or wrong space | 422 if modalities are `text,image` | `T-180-rebuild-model` |
| EC-180-54 | Query image and text together | One aggregated vector hides the image | Two dense legs, then RRF | `T-180-query-both` |
| EC-180-55 | Reranker drops empty captions | Image hits vanish | Keep RRF position when content is empty | `T-180-rerank-image` |
| EC-180-56 | `text,image` on create with `graph` | Half profile | 422 `modalities_require_normal` | `T-180-modalities-graph` |
| EC-180-57 | Direct URL to graph workspace after switcher | Nav from the previous workspace | Capabilities follow the active id | `T-180-ui-switch` |
| EC-180-58 | Trigger bypass via SQL from a bug | Profile flips | Trigger raises; API test uses a raw update in the storage test | `T-180-trigger` |
| EC-180-59 | In-memory workspace service | Postgres tests pass, unit tests forget the column | Memory twin rejects updates | `T-180-memory-immutable` |
| EC-180-60 | Answer prompt | Model claims it sees pixels | Prompt says caption-only; UI shows the image | `T-180-answer-caption` |

## Cross-cutting

- **Idempotent retry of Insert.** A second persist for the same chunk ids replaces terms (`DELETE` postings for those chunk ids, then insert) inside the transaction so `df` does not double.
- **Workspace with zero chunks.** Query returns the existing empty context. BM25 `doc_count = 0` short-circuits the lexical leg.
- **Clearance denies every document.** Kind filter does not widen it. Result is empty.
- **Operator sets `EDGEQUAKE_BM25_BACKEND=pg_search` without the extension.** Process fails at startup with that code, not at the first query.
- **Clock skew on the access token.** Refresh on 401 covers a token that looked unexpired.

# 02 — Research

Parent: [README](README.md) · Prev: [01-first-principles](01-first-principles.md) · Next: [03-product-spec](03-product-spec.md)

Each source is tied to a decision. Sources were read on 2026-10-30. Vendor behavior of `gemini-embedding-2` is confirmed against Google's docs, not against a live call. The live call is wave W0 ([15-implementation-plan](15-implementation-plan.md)).

## R-180-1 — RAG and GraphRAG are complementary

Han et al., "RAG vs. GraphRAG: A Systematic Evaluation and Key Insights" (ACM, 2026).
<https://dl.acm.org/doi/pdf/10.1145/3770855.3817575>

- Plain RAG wins on single-hop and detail questions. GraphRAG wins on multi-hop and on broad summarization.
- On MultiHop-RAG with Llama 3.1 8B, a non-trivial share of questions is answered by only one of the two (about 13.6% GraphRAG-only, 11.6% RAG-only in the reported confusion matrix).
- Graph construction adds cost, latency, and storage, and quality tracks extraction quality.

**Decision:** both profiles exist. The user picks at workspace creation ([LAW-180-1](01-first-principles.md)). This pack does not route one query to both profiles. That routing is a later product, and it needs two workspaces or a graph workspace. Selection-by-query inside one normal workspace is a non-goal because there is no graph to select.

## R-180-2 — Page-level lookup does not want a graph

"Comparing RAG and GraphRAG for Page-Level Retrieval Question Answering on a Math Textbook" (EDM 2026).
<https://educationaldatamining.org/edm2026/proceedings/2026.EDM.poster-demo-papers.386/2026.EDM.poster-demo-papers.386.pdf>

- Embedding RAG beat GraphRAG at retrieving the right page. BM25 beat several embedding models.
- GraphRAG pulled on the order of 47K tokens of context versus about 3.7K for top-5 RAG, and answer quality dropped.
- The paper's reading: GraphRAG is for global and multi-hop questions, not for "which page."

**Decision:** normal retrieval returns chunk context, not community reports. The cost story in [03-product-spec](03-product-spec.md) uses "zero chat-LLM calls at ingest" as the measurable claim ([LAW-180-3](01-first-principles.md)).

GraphRAG-Bench asks the same question from the other side: graph helps when the task needs hierarchy and multi-hop, and it loses when the task does not.
<https://github.com/GraphRAG-Bench/GraphRAG-Benchmark>

## R-180-3 — Hybrid retrieval, then a reranker

"From BM25 to Corrective RAG" (arXiv 2604.01733, 2026).
<https://arxiv.org/html/2604.01733v1>

- RRF with k = 60, unsupervised, no score normalization. It beat CombMNZ and Condorcet in the cited Cormack fusion work.
- On their text-and-table corpus, hybrid RRF then a cross-encoder was the largest jump (Recall@5 0.816 versus 0.695 hybrid alone, 0.644 BM25, 0.587 dense).
- Contextual retrieval at index time is a moderate extra gain. HyDE hurt precise numeric queries.

**Decision:** [LAW-180-6](01-first-principles.md) fixes k = 60 and forbids applying `min_score` to RRF scores. The reranker stays optional and deployment-configured, as today (`EDGEQUAKE_RERANKER`). Contextual chunk prefixes are a follow-up, not this pack. HyDE is out.

EdgeQuake already implements RRF in [fusion.rs](../../edgequake/crates/edgequake-query/src/fusion.rs) line 60 (`RRF_K = 60`) and lines 65–83. Normal retrieval calls that function. It does not invent a second fusion formula.

## R-180-4 — `ts_rank` fused with vectors can lose to vectors alone

"Hybrid search with pgvector got worse until I added BM25" (zeybek.dev, measurements on SciFact and NFCorpus).
<https://zeybek.dev/blog/hybrid-search-in-postgres-measured>

- RRF of `ts_rank_cd` (OR) + vectors scored below vectors alone on SciFact nDCG@10 (0.594 vs 0.703).
- RRF of a real BM25 extension + vectors scored best (about 0.73).
- `websearch_to_tsquery` AND-semantics collapsed recall (SciFact nDCG@10 0.072).

ParadeDB documents the same gap: `ts_rank` has no IDF and cannot answer top-k from the index.
<https://www.paradedb.com/learn/search-in-postgresql/bm25>

Extensions that do implement BM25 (`pg_search`, `pg_textsearch`, VectorChord-bm25) share the access-method name `bm25`, need `shared_preload_libraries`, and are often AGPL or absent on managed Postgres. The EdgeQuake image pins pgvector 0.8.5 only ([extension-pins.sh](../../edgequake/docker/extension-pins.sh)).

**Decision:** [LAW-180-7](01-first-principles.md). Candidate generation may use `tsvector` with OR. Ranking uses BM25 with per-workspace document frequency stored by EdgeQuake. `pg_search` is optional. Graph workspaces are not migrated off `ts_rank_cd` in this pack, because that would change SPEC-046 behavior. The finding is recorded so a later spec can.

W0 measures the built-in BM25 rescore against `ts_rank_cd` on a small public set (SciFact-scale, not a full BEIR run) before the SQL is frozen. If rescore does not beat `ts_rank_cd` on that set, the wave stops and the lexical design is revised. The law (IDF, OR candidates, no required extension) stays.

## R-180-5 — One embedding space for text and image

Gemini Embedding 2, Google AI for Developers (stable id `gemini-embedding-2`, updated April 2026).
<https://ai.google.dev/gemini-api/docs/embeddings>

Vertex multimodal embeddings.
<https://docs.cloud.google.com/gemini-enterprise-agent-platform/models/embeddings/get-multimodal-embeddings>

Facts used by [08-multimodal-embedding](08-multimodal-embedding.md):

| Fact | Value |
|------|--------|
| Model id | `gemini-embedding-2` |
| Vertex method | `:embedContent` (not `:predict`; `:predict` remains for `multimodalembedding@001` and older text models) |
| Default dimension | 3072 |
| MRL | 128–3072. Recommended 768, 1536, 3072 |
| Normalization | Non-default dimensions are L2-normalized by the service |
| Task type | Not supported on this model. Use a prompt prefix for text. Do not put a task prefix on the text part of a mixed image request |
| Text query prefix | `task: search result \| query: …` |
| Document prefix | `title: … \| text: …` |
| Token limit | 8192, shared across modalities. Vendor truncates silently |
| Image | 258 tokens each, at most 6 per request. PNG, JPEG on the Gemini API; Vertex also lists WebP, BMP, HEIC, HEIF, AVIF |
| PDF | 1 file, at most 6 pages, rendered as images (258 tokens per page). 1 page recommended |
| Aggregation | Several parts in one `Content` → one vector. Separate `Content` objects → one vector each |

**Decision:** the repo-side client calls `:embedContent` with `outputDimensionality` set explicitly (768, 1536, or 3072). It never trusts the crate default of 768 in `edgequake-llm` 0.10.9, and it never calls `:predict` for this model. Image limits are enforced before the HTTP call ([LAW-180-4](01-first-principles.md)). The test project is `mobioos-506015`.

## R-180-6 — Page images beat OCR-only retrieval, and we are not building ColPali

ColPali (ICLR 2025) and ViDoRe: embed the page image, late-interaction MaxSim, stronger than OCR pipelines on visual documents.
<https://proceedings.iclr.cc/paper_files/paper/2025/file/99e9e141aafc314f76b0ca3dd66898b3-Paper-Conference.pdf>

ViDoRe V3 (ACL 2026): visual retrievers beat textual ones; late interaction and textual reranking both help; hybrid visual context helps generation. Models still fail on non-text, open questions, and fine grounding.
<https://aclanthology.org/2026.acl-long.755.pdf>

MIDR (EMNLP 2026): index-time multimodal enrichment plus BM25F, fused with dense retrieval, is competitive with ColQwen2.5 on ViDoRe V3 English (0.6219 nDCG@10) at about 9× less index memory and about 2× lower query latency.
<https://aclanthology.org/2026.emnlp-main.1731.pdf>

**Decision:** this pack stores **one vector per chunk or per page image** from `gemini-embedding-2`, plus an optional caption in the BM25 field. It does not store patch multi-vectors or run MaxSim at query time. That is the ColPali non-goal. The MIDR lesson we do take: put visual understanding at index time (the embedding of the page, and a caption for lexical match), and keep query time a text-or-image embed plus BM25.

A survey frame for the taxonomy (retrieval modality and granularity): "Scaling Beyond Context" (ACL 2026, arXiv 2510.15253).

## What we refuse to infer

- We do not claim `gemini-embedding-2` beats ColPali on ViDoRe. No number in this spec says that. W0 and the eval in [16-e2e-test-matrix](16-e2e-test-matrix.md) measure our own corpus.
- We do not claim the built-in BM25 rescore matches ParadeDB. W0 is the gate.
- We do not treat EmbeddingGemma or Gemma 2 as multimodal. The catalog card says `supports_vision = false` ([models.toml](../../edgequake/models.toml) lines 1239–1254).

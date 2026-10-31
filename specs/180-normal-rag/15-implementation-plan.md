# 15 — Implementation plan

Parent: [README](README.md) · Prev: [14-edge-cases](14-edge-cases.md) · Next: [16-e2e-test-matrix](16-e2e-test-matrix.md)

No wave starts until the previous wave's tests pass. W0 can fail the design; it must not leave production code behind if the spike rejects a choice. Spike notes live under `specs/180-normal-rag/spikes/` and are the only write in W0 besides tests that are `#[ignore]`.

## SOLID and DRY, applied per wave

| Rule | How a reviewer checks it |
|------|--------------------------|
| SRP | `NormalRetriever`, `LexicalIndex`, `MultimodalEmbedder`, and `ProfileCapabilities` are separate types. The insert worker does not grow a BM25 formula. |
| OCP | New behavior is a capability flag consumed by existing gates. W2 does not copy `process_text_insert` into `process_text_insert_normal`. |
| LSP | Both `LexicalIndex` backends return the same ranked-id type. |
| ISP | Query code on the normal path does not take a `GraphStorage` parameter. |
| DIP | Vertex HTTP stays behind the trait. The pipeline tests use a fake embedder. |
| DRY | One RRF function ([fusion.rs](../../edgequake/crates/edgequake-query/src/fusion.rs)). One capability struct. One UI constant module. One error code list shared by API and MCP. |

## W0 — Spikes

1. **Vertex embedContent.** From a shell that has run `gcp-use-mobioos`, export `GOOGLE_CLOUD_PROJECT=mobioos-506015` (do not trust an inherited `saas-app-001`). Mint a token with `gcloud auth print-access-token`. Call `:embedContent` for `gemini-embedding-2` with text, with a PNG, and with both. Record HTTP status, vector length at 768, 1536, and 3072, and whether the host is regional or `global`. Record which auth path worked.
2. **BM25 rescore.** On a small public set (SciFact-scale), compare built-in BM25 (OR candidates + IDF) to `ts_rank_cd`. The law stays if BM25 nDCG@10 is at least the vector-fused `ts_rank` baseline in that note. If it is not, stop and revise [09-retrieval-and-fusion](09-retrieval-and-fusion.md) before W3. Do not add `pg_search` to the product image in this pack.
3. **Fake embedder.** Specify the hash-to-unit-vector contract the hermetic tests will use. Implement it in W4, not in W0, unless W2 needs a stub first. W2 text tests can use the existing mock **only inside unit tests** that do not claim to be the Vertex client. The production factory path still fails closed ([LAW-180-4](01-first-principles.md)).

Exit: a spike note with the host, the auth path, the dimension check, and the BM25 table. No migration yet.

## W1 — Profile core

Migration 225 as in [06-data-and-db-contract](06-data-and-db-contract.md): column, checks, trigger, `doc_kind`, `source_modality`, BM25 tables, HNSW 3072, manifest, checksum, schema-train bump.

Code:

- `edgequake-core`: `RagProfile`, `ProfileCapabilities`, fields on create/update requests, Postgres and in-memory services, trigger error mapped to 409.
- `edgequake-api`: create/update DTOs, OpenAPI, error codes for EC-180-01, 03, 04, 05, 56.
- `.env.example`: deprecate `EDGEQUAKE_INGEST_PROFILE` with a pointer to the column.

Tests: `T-180-create-default-graph`, `T-180-create-invalid-profile`, `T-180-update-immutable`, `T-180-update-name-ok`, `T-180-slug-conflict`, `T-180-env-ignored`, `T-180-modalities-graph`, `T-180-trigger`, `T-180-memory-immutable`, `T-180-hnsw-3072` (index present in `pg_indexes`).

Does not ingest a normal document yet. Create returns the column. That is enough.

## W2 — Ingest gates

Touch list, each gated by capabilities ([07-ingest-pipeline](07-ingest-pipeline.md)):

- [workspace_resolver.rs](../../edgequake/crates/edgequake-api/src/processor/workspace_resolver.rs) `judge_ingest`
- [workspace_pipeline_factory.rs](../../edgequake/crates/edgequake-api/src/workspace_pipeline_factory.rs) branch beside line 147
- [processing.rs](../../edgequake/crates/edgequake-pipeline/src/pipeline/processing.rs) line 368
- text_insert prepare / extraction / persist ([persist.rs](../../edgequake/crates/edgequake-api/src/processor/text_insert/persist.rs) 196–217 and 500–509)
- [ingestion_persister.rs](../../edgequake/crates/edgequake-pipeline/src/persistence/ingestion_persister.rs) 792–856
- upload admission for `extraction_mode`
- reprocess mode check
- delete cascade skip
- fingerprint `rp=`
- status mapper messages
- BM25 posting write in the persist transaction, including the workspace lock

Tests: `T-180-ingest-zero-chat`, `T-180-upload-mode-rejected`, `T-180-reprocess-mode`, `T-180-delete-no-graph`, `T-180-no-snapshot-replay`, `T-180-completed-zero-entities`, `T-180-progress-copy`, `T-180-zero-chunks`, `T-180-dedup`, `T-180-no-community`, `T-180-no-summary-judge`, `T-180-cost-fields`, `T-180-df-concurrent`, `T-180-df-delete`, `T-180-scanned-pdf` (text path; image path finishes in W4).

A counting fake chat LLM is the assertion for zero calls. The test fails if summary or extract is constructed.

## W3 — Retrieval

- `LexicalIndex` trait and built-in BM25 in `edgequake-query` / storage SQL.
- `NormalRetriever` calling `reciprocal_rank_fusion` without `min_score`.
- Pipeline branch before keyword LLM when `queries_graph` is false.
- `DocumentFilter` fields, SQL pushdown, clearance AND.
- Query and chat mode checks.
- Optional `pg_search` backend behind the trait, off by default.

Do not change graph-workspace naive or Mix. Regression: existing `contract_bm25_retrieval` and `e2e_spec046_hybrid_rag` stay green.

Tests: `T-180-stopwords`, `T-180-or-query`, `T-180-fts-config`, `T-180-rrf-keeps-hits`, `T-180-dense-only`, `T-180-mode-mix`, `T-180-mode-default`, `T-180-kind-filter`, `T-180-kind-and-clearance`, `T-180-rls-terms`, `T-180-rerank-image` (unit, empty content kept).

## W4 — Multimodal embeddings

- Trait and Vertex client from the W0 note.
- `models.toml` card, `ModelFacts.image_embedding`, catalog DTO.
- Image admission, page-image chunks, token budget, GIF rejection, re-encode.
- Fake `/embedContent`.
- Query image multipart, two-leg fusion (`T-180-query-both`).
- Live `#[ignore]` tests: text-to-text, text-to-image, image-to-image, kind scope. Harness sets `GOOGLE_CLOUD_PROJECT=mobioos-506015`.

Tests: EC-180-18 through EC-180-32, EC-180-53, EC-180-54, `T-180-answer-caption` (prompt fixture contains the caption sentence and does not claim pixels were attached).

## W5 — UI, MCP, i18n

As [13-frontend-architecture](13-frontend-architecture.md) and [10-api-contract](10-api-contract.md) MCP section.

- Wizard step, review, settings card, nav, knowledge empty state, composer chips, query image button.
- Locales and parity test.
- `updateWorkspace` strips profile fields.
- MCP instructions and 409 on entity tools.
- Playwright: `T-180-ui-wizard`, `T-180-ui-query`, `T-180-ui-knowledge`, `T-180-ui-switch`.

## W6 — Verification and docs

- `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings` on touched crates.
- `cargo test -p edgequake-api --test spec027_api_contract`.
- `make codegen-openapi-refresh` if the contract test requires the snapshot.
- `.env.example`, AGENTS env table, `env_registry.toml` for the new weight envs and the deprecation.
- Release notes: PUT that echoes `rag_profile` now returns 409.
- Eval note: same small corpus ingested twice, graph and normal, report ingest chat-LLM calls (expect 0 vs >0), latency, and recall@10. Not a release blocker. Numbers go in `specs/180-normal-rag/spikes/eval.md`.

## Suggested ownership

| Wave | Primary lens |
|------|----------------|
| W0 | AI engineer, RAG expert |
| W1 | Database, full stack |
| W2 | Full stack, AI engineer |
| W3 | RAG expert, database |
| W4 | AI engineer |
| W5 | Front, UX |
| W6 | Product owner |

## Order inside a wave

Schema and pure types first, then the one caller, then the test that would have failed before the caller. Do not update the UI in W2. A normal workspace that exists before W5 is usable through the API.

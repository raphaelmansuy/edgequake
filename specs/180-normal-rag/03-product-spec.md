# 03 — Product spec

Parent: [README](README.md) · Prev: [02-research](02-research.md) · Next: [04-code-findings](04-code-findings.md)

Lens: [LENS-product-owner](lenses/LENS-product-owner.md).

## Problem

EdgeQuake is a Hybrid GraphRAG system. Building the graph is the right spend when questions jump across entities. It is the wrong spend when the job is "find the paragraph or the picture." Operators cannot choose that at workspace creation. They also cannot embed an image as an image, or restrict a search to PDFs or to images.

## Outcome

A workspace is created as **Graph** (default) or **Normal**.

Normal means:

- Ingest embeds chunks and indexes BM25 terms. It does not call a chat LLM.
- Query fuses dense search and BM25, then optionally reranks.
- If the embedding model is multimodal, a text query can return an image, an image query can return an image or a passage, and the user can restrict the search to a document kind.

Graph workspaces behave as they do today.

## Personas

| Persona | Job | Success |
|---------|-----|---------|
| Operator | Create a workspace for a corpus that does not need a graph | Sees the cost difference before create. Cannot accidentally switch later. |
| Analyst | Ask questions over PDFs and screenshots | Gets a cited chunk or image. Can limit the search to images or to PDFs. |
| Admin | Keep graph workspaces for investigations | Existing workspaces do not change profile or reindex. |
| Agent (MCP) | Search a normal workspace | Tools say the workspace has no graph, instead of returning an empty entity list. |

## Stories

1. **Create normal.** Operator opens the create wizard, chooses Normal, keeps a text embedding model, and reviews "No extraction model. Ingest cost is embeddings and storage." Create succeeds. Upload of a markdown file completes with entities hidden and status Completed.
2. **Create normal with images.** Operator chooses Normal and an embedding model that embeds images (`gemini-embedding-2`). The review step says text and images share one index. Upload of a PNG completes. A later text query can return that PNG. A dropped query image can return it too.
3. **Refuse a text-only model for images.** Operator chooses Normal, asks for image embeddings, and picks EmbeddingGemma. Create is blocked with a reason that names the model, not a generic failure.
4. **Graph stays the default.** Operator clicks through without touching the new step. The workspace is Graph. Ingest still extracts.
5. **Cannot convert.** Operator opens settings on a normal workspace. The profile is shown and is not editable. The API returns 409 if a client sends `rag_profile`.
6. **Scope.** Analyst sets the composer scope to Images, or to PDFs. Results and citations come only from that kind, still inside their clearance.
7. **Graph modes.** A client posts `mode=mix` to a normal workspace. The API returns 422 and names the modes that work (`naive`, `bypass`, or omit).

## Cost story (the claim we will measure)

| Stage | Graph workspace | Normal workspace |
|-------|-----------------|------------------|
| Chat LLM at ingest | Extraction, gleaning, summary | None ([LAW-180-3](01-first-principles.md)) |
| Embedding calls | Chunks + entities + relations | Chunks, plus one call per image or page image when modalities include image |
| Vision LLM | PDF pages and figure captions, as today | Same when the operator turns PDF vision or image captions on. Captions are optional and are not required for image search |
| Query LLM | Answer generation, as today | Answer generation only. No keyword-extraction LLM |
| Graph storage | Nodes, edges, communities | None |

Vision for PDF-to-markdown stays available on a normal workspace. It is not graph extraction. The product copy must not say "no LLM" without the word "chat" or "extraction." A scanned PDF can still spend vision tokens. The wizard says so ([12-screens](12-screens.md)).

## Pricing of the choice

The wizard does not invent a dollar figure. It shows counts the server can know:

- Extraction: off, or on (workspace extraction mode).
- Embeddings: text, or text and images.
- Image captions: off by default on normal. On when the operator enables them.

Dollar estimates stay out until a price table exists. A fake price is worse than no price.

## Success metrics

| Metric | Target | How |
|--------|--------|-----|
| Ingest chat-LLM calls on normal | 0 | Counter in [T-180-ingest-zero-chat](16-e2e-test-matrix.md) |
| Graph rows written on normal ingest | 0 | Persist test |
| Existing graph workspace behavior | Unchanged | Default `graph`, regression on SPEC-046 / SPEC-160 tests |
| Text query returns an indexed image | At least one hit in the live eval set | `#[ignore]` Vertex test |
| Kind filter | No hit outside the kind | Contract test |
| Profile change after create | Impossible | API 409 and DB trigger |

Recall@k against a graph workspace on the same corpus is reported, not gated, in the eval note of [16-e2e-test-matrix](16-e2e-test-matrix.md). The product claim is cost and fitness, not "normal beats graph on every question" (R-180-1).

## Out of scope for the operator

- Switching profile in place.
- Choosing a different embedding model per document.
- Audio and video.
- Building a knowledge graph later inside the same workspace.

## Acceptance (product)

- [ ] Create wizard has a Graph / Normal choice, default Graph.
- [ ] Review states extraction off and whether images are embedded.
- [ ] Settings show the profile as read-only.
- [ ] Documents list on a normal workspace does not show entity or relationship counts as if they failed.
- [ ] Query composer can scope by document kind and can attach a query image when the model embeds images.
- [ ] Knowledge graph navigation is not shown.
- [ ] Copy exists in en, fr, and zh.

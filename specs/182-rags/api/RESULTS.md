# API evidence — 24/24

One run of `e2e/evidence.sh` against a live backend, **2026-11-02**, EdgeQuake `v0.39.0`,
Postgres 16, Vertex AI `gemini-embedding-2` at 1536 dimensions, `rag_profile=normal`,
`embedding_modalities=text,image`.

Nothing in this run is stubbed:

- the corpus is 20 real JPEG files uploaded over `POST /documents/upload`;
- every vector is a real `:embedContent` call to `aiplatform.googleapis.com`;
- every ranking below is what the running server returned over HTTP.

Raw artefacts: [`transcript.json`](transcript.json) (each request and response),
[`verdict.json`](verdict.json) (every expectation and its outcome),
[`probe-matrix.json`](probe-matrix.json) (the probe cosine margins).

Reproduce with:

```bash
bash specs/182-rags/e2e/evidence.sh     # starts postgres + backend, runs both suites
```

---

## 1. The workspace states its own capability

`POST /api/v1/tenants/{t}/workspaces`

```json
{
  "name": "Picture library",
  "rag_profile": "normal",
  "embedding_modalities": "text,image",
  "embedding_provider": "vertexai",
  "embedding_model": "gemini-embedding-2",
  "embedding_dimension": 1536
}
```

`201 Created` returns the capability object the client is supposed to read instead of
branching on a profile name:

```json
"rag_profile_capabilities": {
  "extracts_graph": false,
  "embeds_images": true,
  "writes_bm25": true,
  "queries_graph": false,
  "allows_graph_modes": false,
  "shows_graph_ui": false
}
```

That is the whole contract in one object: this workspace stores pictures *and* text in
one vector space, and it has no graph.

`GET /api/v1/models/embedding-support?scope=deployment&provider=vertexai&model=gemini-embedding-2&dimension=1536`
answers the same question for a model pair *before* a workspace exists:

```json
{ "eligible": true, "multimodal": true, "embeds_images": true }
```

Rejected at creation, before any upload:

| Attempt | Result |
|---|---|
| `modalities: "text,image"` with `text-embedding-3-small` | `422 embedding_model_not_multimodal` |
| `gemini-embedding-2` at 1000 dimensions | `422 embedding_dimension_rejected` |
| Vertex with no usable credential | `422 embedding_auth` — never a silent mock |

## 2. Twenty pictures in, twenty image vectors out

All 20 uploads returned `202` and reached `completed`. One chunk per picture, one
vector per chunk, zero chat-model calls — a normal workspace runs no extraction.

```sql
SELECT source_modality, count(*) FROM chunks GROUP BY source_modality;
--  image | 20
```

Total wall clock for tenant + workspace + 20 uploads + 20 ingests: **97 s**, of which
most is the 20 sequential Vertex round-trips.

## 3. Text finds pictures

Human-written question, JSON body, `document_filter.doc_kinds = ["image"]`. The
`score` column is the fused RRF score, not a cosine — see the last section.

| Query | Top hit | Rank 2 |
|---|---|---|
| a path through the woods with sunlight between the trees | `01-forest-path` | `03-desert-road` |
| misty green hills with a road winding through the valley | `02-misty-highlands` | `03-desert-road` |
| an empty highway between red rocks in the desert | `03-desert-road` | `02-misty-highlands` |
| a headshot of a man smiling | `04-man-portrait` | `20-beagle-portrait` |
| a small wooden boat on a turquoise mountain lake | `05-alpine-lake` | `16-orange-poppies` |
| snowy mountain tops above the clouds at sunrise | `06-mountain-peaks` | `03-desert-road` |
| a tropical beach with white sand | `07-tropical-beach` | `17-ocean-wave` |
| a cat staring straight at the camera | `08-cat-portrait` | `13-golden-retriever-puppy` |
| stars and the galaxy over dark mountain silhouettes at night | `09-milky-way` | `01-forest-path` |
| a portrait of a young woman outdoors | `10-woman-portrait` | `07-tropical-beach` |
| a small strong coffee in a cup seen from above | `11-espresso` | `19-cappuccino-latte-art` |
| slices of pizza with melted cheese and basil | `12-pizza-margherita` | `19-cappuccino-latte-art` |
| a young golden retriever puppy | `13-golden-retriever-puppy` | `20-beagle-portrait` |
| the sun going down over a farm field | `14-cornfield-sunset` | `10-woman-portrait` |
| an old white racing bicycle with a leather saddle | `15-vintage-bicycle` | `08-cat-portrait` |
| bright orange wild flowers under a blue sky | `16-orange-poppies` | `14-cornfield-sunset` |
| a wave breaking on the sea at sunset | `17-ocean-wave` | `07-tropical-beach` |
| a modern white building made of sharp geometric shapes | `18-modern-architecture` | `16-orange-poppies` |
| two cups of coffee with heart shapes drawn in the foam | `19-cappuccino-latte-art` | `11-espresso` |
| a happy hound dog panting with its tongue out | `20-beagle-portrait` | `13-golden-retriever-puppy` |

20/20 top-1. Latency **2.6–4.7 s** per query, dominated by the query-embedding call.

The near-misses are the interesting part, and they are all correct: "coffee from
above" returns *espresso* then *cappuccino*; "pizza" returns *pizza* then *cappuccino*;
"orange flowers" returns the flowers then the *sunset field*. The model is separating
Italian food from coffee, and flowers from a sunset, not just grouping "outdoors".

## 4. A picture finds pictures

Multipart `POST /api/v1/query` with `query_image` and an **empty** text query, so the
only signal is the image vector.

| Probe (not in the corpus) | Top hit | Rank 2 | Result |
|---|---|---|---|
| tabby cat on a sofa | `08-cat-portrait` | `13-golden-retriever-puppy` | correct |
| yorkshire terrier puppy on a rug | `13-golden-retriever-puppy` | `20-beagle-portrait` | correct; the cat is rank 4 |
| snow peaks at twilight | `09-milky-way` | `06-mountain-peaks` | correct; both are snow peaks |
| two wood-fired pizzas | `12-pizza-margherita` | `19-cappuccino-latte-art` | correct |

Latency **1.2–1.6 s**. The dog probe puts both dogs above the cat, which is the
distinction a text-only index cannot make at all.

Cosine margins measured directly against the vendor endpoint, before any serving code
([`probe-matrix.json`](probe-matrix.json)):

| Probe | Winner | Score | Runner-up | Margin |
|---|---|---|---|---|
| cat | cat-portrait | 0.769 | retriever puppy | +0.046 |
| dog | retriever puppy | 0.752 | beagle | +0.044 |
| pizza | pizza | 0.813 | cappuccino | +0.059 |
| mountain | milky-way | 0.833 | mountain-peaks | +0.006 *(both correct)* |

The mountain pair is why the acceptance check for that probe is a *set*: two snow-peak
photographs at night are both right answers, and a 0.006 gap is not a ranking.

## 5. It fails closed, and it says why

**Image search on a text-only workspace** — `422`:

```json
{
  "code": "image_query_not_supported",
  "message": "this workspace embeds text only; search by image needs 'text and images'"
}
```

Not a silent fallback to text, which would return an arbitrary picture and look like a
working feature.

**Graph on a chunk-only workspace** — `409`:

```json
{
  "code": "profile_has_no_graph",
  "message": "This workspace has no knowledge graph. It retrieves by embeddings and keywords."
}
```

A chunk index has no entities to draw. Saying so is the feature.

---

## Why the scores look flat

The `score` on a normal workspace is an **RRF fusion score**, not a cosine:

```
score = w_dense/(60 + rank_dense) + w_bm25/(60 + rank_bm25)
```

with `w_dense = 1.0`, `w_bm25 = 1.25`. With at most 20 candidates the denominator is
always 61–80, so the possible scores are the five values `0.01639, 0.01613, 0.01587,
0.01562, 0.01538` and nothing in between. A reader who expects cosine similarity will
think retrieval is broken. It is not: the **order** is the payload, and comparing an
RRF score to a threshold is meaningless. The real cosines are in the table above, and
in `e2e/cache/vectors.json`.

This is not only a documentation footnote. It was two UI defects:

- the image grid printed `score * 100` as a match percentage, showing "2%" beside a
  perfect match and "1%" beside an unrelated picture. `formatMatchPercent` now returns
  `null` below 0.1, so no percentage is shown for a rank score.
- the source-citations badge averaged the same scores, which always landed in the lowest
  band and labelled a perfect match "Mentioned (1%)". It now reports evidence breadth
  (how many distinct sources came back), and shows the band without a number.

## Defects this run found

Every one of these was invisible to a mocked frontend test and appeared only against a
real backend with real vectors.

| # | Symptom | Cause | Fix |
|---|---|---|---|
| 1 | "Matching images" grid never rendered | client tested `file_path` for an image extension; `file_path` holds the document *title*, which has no extension | API states `doc_kind`, `mime_type`, `source_modality` on every citation |
| 2 | Grid vanished again after a reload | those three fields were dropped when the citation context was persisted | persisted on `MessageSource` (same class of bug as `page_start`, SPEC-033) |
| 3 | Send did nothing for a picture-only search | the submit guard required text; the button did not | one tested rule (`composerSubmitIntent`) shared by both |
| 4 | Image probe returned a beagle, not the pizza | the UI sent its display label "Find matches for this image" as the *message*; the server embedded it as a competing text query | `message` (empty) and `display` (label) separated |
| 5 | Same, for every request carrying `language` | `enrich_query_with_language` appended a directive to the empty message, manufacturing a text query | an empty query is left empty |
| 6 | `/health` took 8–11 s and the dev proxy declared a healthy backend dead | the deep document ran a real chat completion, making a local Ollama load a 9 GB model | `list_only` probe: list models, do not generate |

Six defects, all now covered by tests: `file_validation` (5), `message_context_mapper`
(picture facts persisted), `composer-ui-machine` (submit intent),
`use-query-search-image` (message vs display), `edgequake-api::handlers::chat`
(language enrichment), `image-matches` (server-stated picture detection),
`citation-href` (no percentage for a rank score).

# SPEC-180 verification audit — implementation and screens

Date: 2026-11-03. Product pin: v0.39.1. Method: read the pack's own laws, edge-case register and
test matrix, then check each claim against code, a live Postgres, and a real browser. Nothing here
is asserted from memory.

## What was verified green

| Area | Evidence |
|---|---|
| Profile law (LAW-180-1/5) | `t_180_memory_immutable_…` rejects every profile field even unchanged; the DB trigger is now also proven below the API (`t_180_trigger_refuses_a_profile_change_below_the_api`). |
| Capability matrix (LAW-180-2) | `normal::` 36 unit tests: graph never embeds images, normal carries chunks + BM25, graph modes refused. |
| Normal ingest calls no chat LLM (LAW-180-3) | `T-180-ingest-zero-chat` counts chat calls at zero. |
| RRF, not raw scores (LAW-180-6) | `normal::fuse` — consensus beats a single-list win, ties stable across 20 runs, BM25 weight can outrank, and `every_rrf_score_is_kept_even_when_it_is_far_below_a_similarity_threshold` pins the rule itself: k = 60 puts even first place near 0.016, so a `min_score` filter would empty a good result set. |
| Image survives a text reranker (EC-180-55) | `an_image_chunk_survives_the_text_reranker` — the cross-encoder returns only text, and the fused image comes back. The pipeline protects image ids after CE, which is what makes an image-first workspace answerable with rerank on (its default). |
| Dense-only fallback (EC-180-37) | `a_failing_lexical_leg_does_not_fail_the_query`, `backend_without_an_index_serves_dense_only`. |
| Image gating (EC-180-18/22) | 422 `embedding_model_not_multimodal` upload and 422 `image_query_not_supported` query, both refused on a text workspace. |
| One pixel → one chunk (EC-180-19/20/21) | `an_image_becomes_one_pixel_embedded_chunk_and_is_found_by_text_and_by_image`. |
| Screens (11-ux-ui, 12-screens) | 22 live checks and 8 hermetic specs: nav drops graph links, documents shows Chunks and no entity columns, upload hides extraction mode and states the text-only limit, composer offers Chunks + Chat only, `/knowledge` explains itself, the retrieval card is read-only with no control. |

## Defects found and fixed

### 1. BM25 counted documents that have no terms (ranking, silent)

Measured on Postgres 18 against migration 225:

```
image-only workspace (4 picture chunks, content = '')
  bm25_doc_count = 4, bm25_total_len = 0  ->  avgdl = 0.25
```

`avgdl` divides into every length-normalised BM25 score. At 0.25 a three-term chunk has
`len / avgdl = 12`, which multiplies the denominator by ~12 and collapses that chunk. A workspace
that began as pictures and later received text therefore ranked its text badly, with plausible
numbers and no error. The delete trigger had the mirror defect. `rebuild_bm25` shared it
(`count(*) FROM chunks`).

Fixed in **migration 227** (new file, as the immutability rule requires): insert, delete and
rebuild now count only chunks that produced a term, and existing workspaces are repaired in place.
Reproduced after the fix: `avgdl` is 1 for a one-document workspace, and delete returns the
workspace to `0 / 0`.

### 2. Three test-infrastructure bugs that were hiding real coverage

- **The migration-225 SQL had never been executed by a test.** Every assertion compared
  `BM25_SQL` against literal strings. `e2e_spec180_bm25_stats.rs` now runs the triggers, the
  `df` accounting, the regconfig stemming and the rebuild against real Postgres. It was confirmed
  to fail (`left: (4, 0)`) against the old function, so it has teeth.
- **`e2e_spec180_normal_ingest` failed 3/3 on a fresh database** with
  `memberships_user_id_fkey`: tenant creation adds a membership for `USER_ID`, and a database that
  was migrated but never booted through `AppState::ensure_defaults` has no such user. The whole
  suite had been failing on a fixture. The harness now seeds the user row.
- **The image probe cache leaked across tests.** `probe_image` memoises per `(provider, model)`
  process-wide for five minutes, so a test that probed successfully and then pointed at a dead port
  to assert a refusal read the earlier success and saw 201. `start_fake_vertex` now calls
  `forget_all()`, and the test that changes the endpoint clears it too. The assertion also pinned
  `embedding_auth` when an unreachable endpoint legitimately answers `embedding_request_failed`;
  it now accepts either, because both refuse the create.

### 3. Two laws had no test at all, though the behaviour was right

LAW-180-6's "`min_score` is not applied to RRF scores" is the mitigation for finding F-180-12,
and nothing asserted it: `fuse_ranked` does not filter, but a future edit adding a threshold would
have passed the suite. EC-180-55's image protection existed in `reranking.rs` and was likewise
unasserted. Both now have a test, and the F-180-12 one is written so it fails if anybody adds a
similarity filter after fusion (it asserts a real tail score below 0.1).

I also suspected EC-180-55 was broken, because image chunks are retrieved by dense search and the
dense SQL does not select `source_modality`. That was wrong, and worth recording: the protection
reads `chunks.metadata->>'source_modality'`, which the ingest writer *does* populate — verified on
the live database, where every image chunk carries it.

### 4. The test matrix does not match the test names

The matrix in [16-e2e-test-matrix](16-e2e-test-matrix.md) names ~45 tests. The code uses
different names for the same coverage (`T-180-mode-422` for EC-180-38, `T-180-mcp-entity-409` for
EC-180-44, and the image file uses prose names). The *coverage* is real; the *index* is stale, so
a reader cannot trace EC → test by grep. [18-verification](18-verification.md) records the mapping
that actually holds.

## Known open items

| Item | Why it is open |
|---|---|
| EC-180-33 (stopword-only query) | Behaviour is correct — `blank_text_skips_the_lexical_leg` and the DB `qterms` CTE both yield an empty lexical leg — but there is no test named for a query of *only* stopwords. |
| EC-180-34 (OR semantics) | Verified by reading `BM25_SQL` (it never emits `&`, and `qterms` is a `DISTINCT` union) and by `sql_uses_the_documented_constants_and_or_semantics`, but no test runs a long question whose rare term appears in one chunk. |
| EC-180-51/52 (concurrent `df`, delete) | The workspace row lock and the delete path are exercised, but no test runs two concurrent ingests to prove lost-update safety. |
| Live Vertex pair (`T-180-text-to-image_live`) | Requires `mobioos-506015` and a token; not run here. CI does not call Vertex by design. |

None of these is a shipped-behaviour defect; they are coverage debt recorded rather than hidden.

## SPEC-183 follow-up — four defects found by reading two screenshots

Prompted by two screenshots of the same 20-picture Normal workspace: an answer that refused
although retrieval had found everything, and default questions that no picture could answer.

### 1. RRF scores were printed to the model as relevance

`format_chunk_block` wrote `score: {:.3}`. On a Normal workspace that value is a reciprocal
rank, so every retrieved chunk was announced as `score: 0.016` — identical, meaningless, and
below every threshold a model might reason about. **The dense cosine was computed and then
thrown away by fusion.** It is now carried through as `RetrievedChunk::similarity`, persisted on
`MessageSource`, exposed as `SourceReference::similarity`, and the prompt prints
`relevance: fused by rank` instead of a fake number. Measured separation on the live corpus:

| Question | Top similarity |
|---|---|
| "a golden retriever puppy" | 0.715 |
| "coffee with latte art" | 0.696 |
| "an alpine lake with mountains" | 0.650 |
| "what does the handbook say about refunds?" | 0.473 |

### 2. A refusal was labelled "Strong"

`calculateConfidence` scored retrieval breadth, so 20 chunks produced `1.0` → "Strong" directly
above *"there is insufficient evidence"*. `calculateAnswerConfidence` now reads the answer and
returns `null` on a refusal, and the badge is not rendered at all. The refusal matcher handles
passive voice ("cannot be determined"), which is the phrasing the live model actually produced.

### 3. The model was never told what an image placeholder meant

The prompt said "answer ONLY from the Context" and the context said `[Image: 05-alpine-lake]`,
so refusing was the correct behaviour. `grounding::image_context_instructions()` is appended
**only when the context holds imageless picture chunks**, and tells the model it can name the
pictures and state the limit but must never describe pixels. Live effect: the same question now
returns *"The workspace contains 20 retrieved images [13-golden-retriever-puppy] …"* instead of
an apology, and still refuses to describe the alpine lake.

### 4. Default questions could not be answered by the corpus that showed them

`NORMAL_COPY.suggestions` was a hardcoded list of text-document questions ("What does the
handbook say about refunds?") shown to every Normal workspace. `getQueryEmptyCopy` now takes a
`CorpusShape`, and a workspace that embeds pictures and holds more pictures than text gets
picture questions. Live: the 20-photograph workspace now greets with **"Search your pictures"**.

### 5. The image grid ignored relevance entirely

Reported after the first four: *"don't display image where rank is too low."* The grid rendered
every image source, and `formatMatchPercent` returned `null` for any score under 0.1 — which is
every fused score — so no percentage appeared and nothing was filtered. The gate is now two-part,
because an absolute floor alone cannot work: the runner-up of a *good* query (0.610) is no higher
than the runner-up of a query with no answer at all (0.467).

```text
MIN_IMAGE_SIMILARITY    = 0.48   absolute floor; unrelated pairs never exceed ~0.48
IMAGE_SIMILARITY_MARGIN = 0.15   relative to the best match
```

Both values were chosen from the measurements above, not guessed: 0.15 keeps the retriever
(0.715), beagle (0.610) and poppies (0.569), stops at the ocean wave (0.530), and hides all
twenty pictures for the refund question. 0.10 was tried first and dropped the beagle; 0.20
readmitted the bicycle. The rule fails **open** when `similarity` is absent, so stored
conversations from before this field keep their tiles.

Live result: refund question → no grid at all. "a golden retriever puppy" → 3 tiles, 17 counted
as weak in the heading rather than shown.

### Test coverage added

Rust: `fusion_keeps_the_dense_similarity_it_used_to_discard`,
`two_dense_lists_keep_the_better_similarity`, `a_fused_rank_score_is_not_presented_as_relevance`,
`a_real_similarity_keeps_its_number`, `the_rank_band_boundary_is_where_rrf_stops`,
`an_image_context_tells_the_model_what_the_placeholders_mean` (renders the whole prompt),
`a_text_context_is_unchanged_by_this_pack`, plus the grounding tests.
TypeScript: 15 image-gate cases and 5 answer-confidence cases driven by the measured numbers.

## Closing a pre-existing red: RRF filtered by a cosine floor

`contract_query_modes::mix_with_skewed_weights_differs_from_hybrid` had been failing on every
checkout, dismissed twice as "unrelated". It was not noise. Tracing it found a live violation of
LAW-180-6 in `sparse_retrieval::fuse_vector_and_bm25_chunks`:

```rust
chunks.retain(|c| c.score >= min_score);   // min_score = 0.1, RRF scores ≈ 0.0164
```

On the RRF path every score is `1/(60+rank)`, so the cosine floor (0.1) discarded **every** fused
hit. Any graph query whose BM25 leg produced a candidate therefore returned nothing. The fix
applies the floor only on the non-RRF branch, where the scores really are similarities; the legs
already enforce their own floors.

The test that should have caught it read its own source:

```rust
let src = include_str!("sparse_retrieval.rs");
assert!(src.contains("chunks.retain(|c| c.score >= min_score)"));
```

That asserts the *text of the bug*, not behaviour, so it passed for as long as the bug existed.
It is replaced with `rrf_fusion_keeps_every_hit_even_below_the_cosine_floor`, which builds a real
fused list and asserts nothing is dropped.

Three further stale fixtures were making correct code look broken, all the same pattern — a
fixture that encodes an impossible expectation:

| Test | Was failing because |
|---|---|
| `mix_with_skewed_weights…` | Entity chunk vector had cosine 0.0255 with the mock query, i.e. genuinely irrelevant; the test required it to be returned, which only a missing floor allows. Fixture now scores above the floor. |
| `contract_rerank_stats` ×2 | Asserted on `query_execute.rs` / `completion.rs`, but the forwarding moved to `query_stats_mapper.rs` and `context_bundle_mapper.rs`. Now asserts the real home. |
| `spec103_query_answer_hit_skips_generate` | Required `keyword_cache_hit` on a `Naive` query, which never runs keyword extraction. |

Still red and **not** touched by this work: `contract_streaming_vision::streaming_query_without_images_uses_text_stream_path`
(zero `stream()` calls against its fixture; reproduced on a clean checkout). It needs its own
investigation.

## Suggestions for a classical RAG that is not only text

The first fix branched on "are most documents pictures?", which still treats a corpus as one
thing. A classical RAG corpus is usually **mixed** — a brand manual beside its product
photographs — and that rule gave it a single-kind greeting, so the manual was unreachable from the
empty screen.

Replaced with capability-driven composition. Each suggestion declares the capability it needs
(`text`, `pdf`, `image`, `office`); the corpus reports which it has; suggestions are drawn
round-robin from the pools those unlock, so a corpus that is 98% photographs still offers a text
question in its four slots. A suggestion is printed only if the corpus can keep its promise.

Three heads rather than two, because "I find the passages… point to the page" is wrong for a
corpus with photographs and "Search your pictures" is wrong for one with a manual:

| Corpus | Head |
|---|---|
| text/PDF/office only | Ask about your documents |
| pictures only | Search your pictures |
| both | **Search your documents and pictures** |

Measured on a real mixed workspace (1 image + 1 markdown + 1 txt, uploaded through the API):

```text
HEADING: "Search your documents and pictures"
SUGGESTIONS: ["What does policy say about refunds?",   <- names a real title
              "Which pictures are in this workspace?",
              "Find the passage that mentions deadlines",
              "Find a picture with a body of water"]
```

**A defect this found in its own first attempt:** the screen rendered a literal `{{topic}}`. The
lib substituted the placeholder in the English fallback, but the component renders
`t(key, fallback, opts)`, whose *translated* value carries the placeholder too. Each side was
correct in isolation. `topicForCorpus` is now the single substitution point, and a test asserts no
suggestion can carry an unresolved placeholder.

Coverage: 16 cases in `lib/query/__tests__/query-empty-copy.test.ts` — capability derivation,
mixed interleaving, no picture questions on a workspace that cannot embed images, the
placeholder-leak guard, topic naming, and family classification by `source_type` → MIME →
extension.


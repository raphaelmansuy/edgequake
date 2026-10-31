# 08 — Multimodal embedding

Parent: [README](README.md) · Prev: [07-ingest-pipeline](07-ingest-pipeline.md) · Next: [09-retrieval-and-fusion](09-retrieval-and-fusion.md)

Lens: [LENS-ai-engineer](lenses/LENS-ai-engineer.md). Research: [R-180-5](02-research.md), [R-180-6](02-research.md).

## Trait

In `edgequake-core` (or a small module under `edgequake-pipeline` if the HTTP client would pull Axum — it must not). The HTTP client sits in `edgequake-api` or a new file under `edgequake-llm` **inside this repo** is wrong: `crates/edgequake-llm` is not the crate. **Home: `edgequake-api/src/embeddings/vertex_gemini.rs` behind a trait in `edgequake-core`.**

```text
enum EmbedPart { Text(String), Image { mime, bytes } }

trait MultimodalEmbedder {
  fn model(&self) -> &str
  fn dimension(&self) -> usize
  async fn embed(&self, inputs: &[EmbedInput]) -> Result<Vec<Vec<f32>>>
}

struct EmbedInput { parts: Vec<EmbedPart> }  // one input → one vector
```

`EmbeddingProvider::embed(&[String])` stays for text-only models. A normal workspace whose modalities are `text` uses that trait, including EmbeddingGemma via Ollama. A workspace whose modalities are `text,image` uses `MultimodalEmbedder` only. The factory does not wrap one in the other by dropping images.

Text-only path for `gemini-embedding-2` still goes through `MultimodalEmbedder`, because the crates.io client calls `:predict` ([F-180-17](04-code-findings.md)). Any workspace that selects this model uses the new client, even if modalities are `text`.

## Vertex request

`POST https://{region}-aiplatform.googleapis.com/v1/projects/{project}/locations/{region}/publishers/google/models/gemini-embedding-2:embedContent`

W0 confirms whether this model requires `location=global`. If the regional host returns 404 or `FAILED_PRECONDITION`, the client uses `aiplatform.googleapis.com` with `locations/global` and records the result in the spike note. The test does not guess in production code: the spike output is a constant in the client, with the region env `GOOGLE_CLOUD_REGION` honored when the spike says regional works.

Body (one input):

```json
{
  "content": {
    "parts": [
      { "text": "title: policy | text: …" },
      { "inlineData": { "mimeType": "image/png", "data": "<base64>" } }
    ]
  },
  "embedContentConfig": { "outputDimensionality": 1536 }
}
```

Batch: one HTTP call per input. The vendor's embed endpoint for this model is not the old `:predict` batch. The client fans out with the existing embedding concurrency limit (`EDGEQUAKE_EMBEDDING_BATCH_SIZE` is a misnomer here; use a dedicated concurrency cap, default 4). Do not send `taskType`.

### Prefixes

| Input | Text part |
|-------|-----------|
| Document or chunk | `title: {doc title} \| text: {chunk}` |
| Query text | `task: search result \| query: {query}` |
| Image only | no text part |
| Image plus caption or page text | image part, then text part **without** a task prefix (R-180-5) |

Titles longer than 200 characters are truncated in the prefix only. The chunk body is still subject to the token budget.

### Dimension

Create-time allow-list: 768, 1536, 3072 ([LAW-180-5](01-first-principles.md)). The client always sends `outputDimensionality`. It does not rely on the vendor default 3072. Returned length must equal the workspace dimension or the call fails `embedding_dimension_mismatch`. Vectors are L2-normalized again in the client if the norm is not already 1 ± 1e-3, so cosine in pgvector matches the vendor note that non-default widths are normalized.

### Limits enforced before HTTP

| Limit | Behavior |
|-------|----------|
| More than 6 images in one input | Split into several inputs (several vectors). A single chunk never has more than 6 images. Page chunks have one image. |
| Image bytes above `EDGEQUAKE_VISION_MAX_IMAGE_BYTES` (default 3_500_000) | Re-encode to JPEG, same helper family as SPEC-134. If still over, fail `image_too_large`. |
| Side above 16384 px | Fail `image_too_large`. Do not send. |
| MIME not in png, jpeg, webp | Transcode to jpeg when the decoder accepts the file. GIF and animated WebP: **reject** `image_animated_unsupported` (first-frame embedding would surprise the user). |
| Corrupt bytes | `image_decode_failed`. |
| Estimated tokens above 8192 | Split text, or store image and text as two chunks. Never send and hope. Token estimate: 258 per image + ceil(chars / 4) for text. Conservative on purpose. |
| PDF sent as a raw PDF part | Not in this pack. Pages are rasters the PDF pipeline already writes. |

## Auth

- Project: `GOOGLE_CLOUD_PROJECT`. Tests set `mobioos-506015`. A developer shell that exports a different project must not leak into the test ([LAW-180-15](01-first-principles.md)).
- Token: `GOOGLE_ACCESS_TOKEN` if set, else `gcloud auth print-access-token` using the active account (`vertex-express@mobioos-506015.iam.gserviceaccount.com` after `gcp-use-mobioos`). Cache the token and refresh when less than 5 minutes remain, or on HTTP 401 (one retry).
- Missing project or token: error `embedding_auth` at provider construction. **No MockProvider.** The existing `ensure_non_mock_provider` guard stays, and the new client never calls the factory branch that substitutes a mock ([F-180-17](04-code-findings.md)).
- `GOOGLE_APPLICATION_CREDENTIALS` is not sufficient by itself until a spike proves the client can mint a token from that file. W0 records which of ADC file, `gcloud` print, or metadata server worked. The client implements the one that worked, plus refresh. It does not document a path it cannot execute.

## Catalog and policy

[models.toml](../../edgequake/models.toml) gains a Vertex card:

- name `gemini-embedding-2`
- model_type `embedding`
- `embedding_dimension` 1536 as the card default (create can still pick 768 or 3072)
- new capability `embeds_image = true`

`ModelFacts` gains `image_embedding: bool`. `supports(Usage::ImageEmbedding)` is that flag. Create and rebuild call it. Text embedding usage stays `Usage::Embedding`.

`Workspace::known_embedding_dimension` must learn this model. Until it does, an unknown name falls through to 1536 ([F-180-18](04-code-findings.md) neighborhood in workspace.rs). The card is the source of truth on the write path (SPEC-178). The fallback table is updated so a missed card does not store 1536 while the client requests 3072, or the reverse: the client dimension is the workspace dimension, and the stored vector is checked against it.

Discovery that returns this model without a dimension does not make it selectable for `text,image` until the static card fills `embeds_image`. A discovered id alone is not evidence it embeds images.

## Errors

| Code | HTTP | When |
|------|------|------|
| `embedding_model_not_multimodal` | 422 | modalities `text,image` and card has `embeds_image = false` |
| `embedding_dimension_rejected` | 422 | dimension not in 768, 1536, 3072 |
| `embedding_auth` | 503 on ingest, 422 on create if the probe fails | no credential |
| `embedding_dimension_mismatch` | 500 on ingest | vendor returned the wrong width |
| `image_too_large` | 422 | pixels or bytes |
| `image_animated_unsupported` | 422 | GIF, animated WebP |
| `image_decode_failed` | 422 | corrupt |
| `image_query_not_supported` | 422 | query image when modalities are `text` |

Create probes one short text embed when the provider is Vertex, so a bad project fails in the wizard instead of on the first PDF. The probe is skipped when `EDGEQUAKE_SKIP_EMBED_PROBE=1` (tests).

## Fake server

`edgequake-fake-llm` grows `POST /embedContent` that returns a deterministic unit vector of the requested dimension from a hash of the parts. Hermetic tests never call Google. The live test is `#[ignore]`.

## Non-goals

- `multimodalembedding@001` (`:predict`, 1408).
- Audio, video, raw PDF parts.
- Patch-level late interaction (R-180-6).
- Upstream release of `edgequake-llm`. A follow-up issue describes the `:embedContent` gap so the fork does not rot.

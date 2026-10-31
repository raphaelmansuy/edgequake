# Lens — AI engineer

Parent: [README](../README.md) · Spec: [08-multimodal-embedding](../08-multimodal-embedding.md)

## View

`gemini-embedding-2` is the multimodal model. Gemma 2 and EmbeddingGemma are not. The crates.io client cannot call this model: it uses `:predict`, hard-codes a task type the model rejects, assumes 768 dimensions, drops the dimension argument, and substitutes a mock when auth is missing ([F-180-17](../04-code-findings.md)).

The new client sends `:embedContent`, sets `outputDimensionality`, uses the documented prefixes, and fails closed. One input is one vector. Six images is a hard cap. Over-long input is split before the call because the vendor truncates silently.

Auth is whatever W0 proved, with refresh. Tests pin project `mobioos-506015`.

## Risks

| Risk | What we do |
|------|------------|
| Regional endpoint 404 | W0 picks `global` or the region and the client encodes that result. |
| Token dies after an hour | Cache, refresh under 5 minutes, retry once on 401. |
| Card dimension 1536 but a caller expected 3072 | Allow-list is explicit. The client checks the returned length. |
| Discovery lists the model without `embeds_image` | Static card is required for image modalities. |
| Page text plus page image exceeds 8192 tokens | Two chunks, not one truncated vector. |

## Acceptance

- Fake `/embedContent` covers CI.
- One ignored live run records text-to-image and image-to-image.
- Production path has no mock fallback.
- Follow-up issue text for upstream `edgequake-llm` is in the W4 PR description.

## Laws owned

LAW-180-3, LAW-180-4, LAW-180-5, LAW-180-9. WHY-B.

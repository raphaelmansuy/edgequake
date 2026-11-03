# SPEC-183 — Reconciling a Normal workspace with its own screens

Status: **fixed**. Two defects, one cause each, both reproduced in a browser before the change
and re-proved after it against the running stack.

## What the user reported

> Edit doesn't work. Image cannot be uploaded.

## Defect 1 — a live-looking Edit link that did nothing

`stepsForWizard('reconfigure-workspace', { ragProfile: 'normal' })` returns
`['models', 'document-parsing', 'chunking', 'review']`. There is no `retrieval` step, and there
must not be: the profile is immutable (LAW-180-1).

`ReviewStep` was mounted with `showRetrieval`, so it printed a Retrieval group whose Edit button
called `goToStep('retrieval')`. That looked the index up in a list that did not contain it, got
`-1`, and silently did nothing. Measured before the fix:

```
wizard-review-retrieval-edit:   landed on "Review and apply" -> DEAD
wizard-review-models-edit:      landed on "Confirm AI models" -> ok
wizard-review-document-parsing-edit: landed on "Document parsing" -> ok
wizard-review-chunking-edit:    landed on "Chunking" -> ok
```

One dead control among working ones is worse than a missing control: it reads as a broken wizard
rather than as a step that does not exist. The same shape produced the reported symptom — Apply
stayed disabled behind "Nothing has changed yet. Edit a step to apply." with the one Edit link a
user would reach for being inert.

**Fix.** `editHandlerFor(steps, step, goToStep)` returns a handler only when the step is in the
wizard's list, and `ReviewStep` now requires `steps`. A caller cannot reintroduce the dead link by
forgetting to pass the list. The Retrieval *facts* stay on the review — only the unusable button
is gone, so the choice, the evidence and its permanence are still readable.

## Defect 2 — images were unreachable, not merely refused

The server was right: a `text` workspace refuses an image with
`embedding_model_not_multimodal` (EC-180-18 — an image is never captioned in its place).

The unreachable part: `embedding_modalities` is create-only and immutable, and the create dialog
(SPEC-181 W-03) asked name, retrieval type and models — not evidence. Measured before the fix:

```
POST /workspaces  {"name":"Image probe repro","slug":"image-probe-repro","rag_profile":"normal"}
```

No `embedding_modalities`, so every workspace a person could create from the UI was `text`. With
the column immutable there was no control anywhere that could ever admit a picture, on any screen,
for the life of the workspace. The refusal was correct and the product was still wrong.

**Fix.** The Evidence decision now sits beside the retrieval type, because both are fixed at
create and so both have to be asked before create:

- `resolveCreateModalities` clamps a graph workspace to `text`, so the dialog can never POST a
  pair the server refuses (`modalities_require_normal`).
- `imageEmbeddingPin` selects an image-capable embedding model when images are chosen and the
  inherited one reads text only — otherwise the operator lands on a disabled Create button beside
  a text-only embedder, which is the same dead end one screen later.
- When nothing offered can read images, the card and the footer both say so, and *Text* stays
  reachable. An honest dead end beats a button that fails.

Then the dropzone half of the same story: a text-only workspace says so before the drop, and a
dropped picture is refused at the edge with the destination named (a new workspace with images).

The guard fails **open**: only `embeds_images: false` reported by the server blocks. An absent
capability is "unknown", not "no" — otherwise a surface whose payload predates the field would
refuse uploads the deployment accepts.

## Proof after the fix, against the running stack

```
dead Retrieval Edit link present: 0     (Models / Parsing / Chunking: 3 links, all navigate)
evidence guidance: model named
Create enabled: true
POST: {"rag_profile":"normal","embedding_modalities":"text,image",
       "embedding_provider":"vertexai","embedding_model":"gemini-embedding-2"}
server: modalities text,image · embeds_images true
upload: 202 accepted
ingest: live-probe -> completed, 1 chunk
```

The last line matters for LAW-180-3: the picture became one chunk and one embedding with no chat
LLM call. And the text-only workspace still refuses it, correctly:

```
{"code":"embedding_model_not_multimodal","http_status":422}
```

## Tests

| Layer | What |
|---|---|
| `lib/onboarding/__tests__/review-edit.test.ts` | An Edit link exists only for a reachable step. |
| `lib/admin/__tests__/create-evidence.test.ts` | Graph clamps to text; the pin chooses an image embedder; no guess when none is offered. |
| `lib/documents/__tests__/image-upload-guidance.test.ts` | The sentence names the fix; unknown capability refuses nothing. |
| `lib/upload/__tests__/file-kind.test.ts` | A drop splits into what can be stored and what cannot. |
| `e2e/spec183-workspace-images.spec.ts` | 8 specs, both defects, plus the honest dead ends. |

Screenshots: [`screenshots/round-01`](screenshots/round-01). Suite state: 2320 unit tests pass;
59 neighbouring e2e specs (SPEC-180, SPEC-181, image upload, dropzone audit) still pass.

## What this pack does not change

`embedding_modalities` stays immutable, and an existing text-only workspace still cannot be
converted. That is LAW-180-1/5 and the pack deliberately keeps it: the fix is that the choice is
now *askable*, not that it becomes mutable. A user who already created a text-only workspace has
to make a new one — which the dropzone, the review and the create card all now say out loud.

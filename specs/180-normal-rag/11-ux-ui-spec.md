# 11 — UX and UI spec

Parent: [README](README.md) · Prev: [10-api-contract](10-api-contract.md) · Next: [12-screens](12-screens.md)

Lenses: [LENS-ux-ui](lenses/LENS-ux-ui.md), [LENS-front](lenses/LENS-front.md).

Wireframes: [12-screens](12-screens.md).

## Principles

- The profile is a structural choice, shown before model pick, because the model list depends on it (image embeddings filter the catalog).
- Default is Graph. The first radio is Graph, selected. Normal is the second, not a buried advanced toggle.
- Immutable fields look like text, not like disabled inputs. A disabled radio still looks editable. Read-only copy does not.
- Do not say "BM25" in the primary label. Say "keyword match." BM25 can appear in a secondary sentence for operators who know the term.
- Do not say "no LLM" without qualification. Say "no extraction model." Vision for PDFs stays a separate sentence.
- Graph pages are removed from navigation. A direct URL shows an explanation and a link back to documents, not a broken graph.

## Wizard

Insert step `retrieval` after `workspace-basics` and before `models` in `stepsForWizard` for `create-workspace` ([wizard-state.ts](../../edgequake_webui/src/lib/onboarding/wizard-state.ts) lines 194–195).

| Step | Graph | Normal |
|------|-------|--------|
| Basics | name, slug | same |
| Retrieval | profile, modalities, language of keywords | same |
| Models | chat, embedding, vision as today | chat model optional in the copy; embedding required; vision only if PDF parsing or captions are on |
| Document parsing | as today | as today, plus "page images are embedded" when modalities include image |
| Chunking | as today | as today |
| Extract budget | shown | skipped |
| Extraction | shown | skipped |
| Review | graph summary | "Extraction off. Keyword index on. Images on or off." |

`wizardBlocker` for `retrieval`: if modalities are `text,image` and the later model step has no eligible image embedding model, the model step blocks with `embedding_model_not_multimodal`. The retrieval step itself only blocks on an empty selection, which cannot happen because Graph is preselected.

Review is the last place that states immutability: "You cannot switch this workspace to the other retrieval type later."

## Settings

A card `WorkspaceRetrievalCard` on the workspace page, above extraction.

- Shows profile, modalities, keyword language.
- No edit control.
- One sentence: "To use the other type, create a workspace."
- Extraction card, extract budget, and rebuild-knowledge-graph button render only when `shows_graph_ui`.
- Rebuild embeddings stays.

## Documents

- Hide entity count and relationship count when `extracts_graph` is false. Do not show "0 entities" in red.
- Status `completed` with zero entities is success.
- Progress strings come from the server. The client does not substitute "Extracting entities" for a stage it does not understand; it shows the server message.
- Upload: hide the extraction-mode select on a normal workspace. If a stale client sends it, the server 400 is shown as the server message.
- Image drop is already an upload. On a text-only workspace the dropzone rejects `image/*` with the multimodal message before the request.

## Query composer

- Mode menu: on normal, only Chunks (`naive`) and Chat (`bypass`). Hide Focused, Broad, Linked, Smart. Default Chunks.
- Scope chips gain a kind chip: All, PDFs, Images, Markdown, Text, Office. Multi-select. Empty means all.
- Image query: a paperclip already exists for mentions. Add "Search by image" that attaches one image. Hidden when `embeds_images` is false.
- Companion graph panes: not mounted when `shows_graph_ui` is false.

## Knowledge graph route

Direct navigation to `/knowledge` (and the slug-scoped twin) when the active workspace is normal:

```text
This workspace has no knowledge graph.
It retrieves by embeddings and keywords.
[ Go to documents ]  [ New graph workspace ]
```

No empty canvas, no spinner.

## MCP and agents

Out of the visual UI, but the same copy rule: 409 message is the sentence above, plus the code `profile_has_no_graph`.

## Internationalization

Keys under `ragProfile.*` in `en.json`, `fr.json`, `zh.json`. A parity test copies `extraction-mode-locales.test.ts`. No hardcoded English in the wizard step.

## Accessibility

- Radios are a fieldset with a legend "Retrieval type."
- Kind chips are toggle buttons with `aria-pressed`.
- The read-only card is a definition list, not a form.
- Errors from 409 and 422 are in an `alert` region on the wizard and on the composer.

## States

| State | UI |
|-------|----|
| Loading capabilities | Skeleton on the card. Do not flash the graph nav and then hide it. Gate nav on the workspace query. |
| Workspace list mixed | Nav follows the **active** workspace, not the tenant. |
| Create probe failing | Inline error on the model step, from `embedding_auth`. Do not advance to review. |
| Query image too large | Composer error before send. |
| Filter matches nothing | Existing empty result. Do not suggest that the index is broken. |

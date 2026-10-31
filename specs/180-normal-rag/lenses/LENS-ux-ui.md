# Lens — UX and UI

Parent: [README](../README.md) · Spec: [11-ux-ui-spec](../11-ux-ui-spec.md), [12-screens](../12-screens.md)

## View

The user is choosing a structure. The screen says what they will not get (no graph, no extraction model) and what they still pay (embeddings, optional PDF vision). Permanence is a sentence on the review step, not a tooltip.

Graph chrome disappears. A zero entity count shown in the documents table would teach the user that ingest failed. Those columns are removed, not zeroed.

Kind chips answer "only images" and "only PDFs" without a second query language.

## Risks

| Risk | What we do |
|------|------------|
| Disabled radios look broken | Read-only settings use a definition list. |
| Nav flashes the graph item | Nav waits for workspace capabilities. |
| Mode menu still offers Smart (Mix) | `modesForProfile` hides it. Server rejects it anyway. |
| "Keyword match" is vague | Secondary line may say BM25. Primary label does not. |

## Acceptance

- Wireframes in [12-screens](../12-screens.md) match the built steps: retrieval, review, settings, documents, composer, knowledge empty state.
- Copy exists in en, fr, zh.
- Empty knowledge state has two actions: documents, and create a graph workspace.

## Laws owned

LAW-180-10, LAW-180-11. WHY-E.

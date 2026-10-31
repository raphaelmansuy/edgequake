# 13 — Frontend architecture

Parent: [README](README.md) · Prev: [12-screens](12-screens.md) · Next: [14-edge-cases](14-edge-cases.md)

Lens: [LENS-front](lenses/LENS-front.md).

## Single sources

| Concern | File | Rule |
|---------|------|------|
| Wire values | `src/constants/rag-profile.ts` | `graph`, `normal`, modalities, kinds. No second array in a component. |
| Payload | `src/lib/onboarding/model-payload.ts` `buildWorkspaceIngestPayload` | Adds `rag_profile`, `embedding_modalities`, `fts_regconfig` on **create** only. Update payloads omit them. |
| Draft | `WizardDraft` in `wizard-state.ts` | New fields with defaults `graph`, `text`, `english`. |
| Diff | `workspace-config-diff.ts` | Profile is not a diffable key. Reconfigure wizard does not show it as a change. |
| Capabilities | `Workspace.rag_profile_capabilities` from the API | Components read booleans. They do not compare `rag_profile === 'normal'` except the settings card that prints the label. |
| Modes | `query-mode-meta.ts` | `modesForProfile(caps)` filters `QUERY_MODE_META`. |
| i18n | `locales/{en,fr,zh}.json` | Parity test. |

This is the DRY rule: the capability object is the SPEC-160 extraction view, repeated for this profile, not a parallel boolean computed in React.

## Components

| Component | Action |
|-----------|--------|
| `steps/workspace-retrieval-step.tsx` | New. Radios. Uses constants and i18n. |
| `steps/review-step.tsx` | Renders the retrieval lines from the draft. |
| `steps/workspace-extraction-step.tsx` | Unchanged. Not mounted when the draft is normal. |
| `workspace/workspace-retrieval-card.tsx` | New read-only card. |
| `workspace/workspace-extraction-mode-card.tsx` | Mounted only if `extracts_graph`. |
| `workspace/rebuild-knowledge-graph-button.tsx` | Mounted only if `extracts_graph`. |
| `documents/upload-extraction-mode-select.tsx` | Not mounted when `extracts_graph` is false. |
| `query/composer/mode-menu.tsx` | Options from `modesForProfile`. |
| `query/composer/kind-chips.tsx` | New. Writes `doc_kinds` into the chat request builder. |
| `query/composer/query-image-button.tsx` | New. Hidden without `embeds_images`. |
| `lib/query/build-chat-request.ts` | Adds kinds, mime, source modalities, and omits `mode` when the user left the default on a normal workspace (server applies normal retrieval). Sending `naive` is also acceptable. Do not send `mix`. |
| Knowledge page | Early return empty state from [12-screens](12-screens.md) when `shows_graph_ui` is false. |
| Nav | Filter the knowledge item with the same flag. |

## Store

`use-settings-store` default `mode: "mix"` is wrong for a normal workspace ([F-180-09](04-code-findings.md)). The composer initializes mode from the active workspace: `mix` for graph, `naive` for normal. Persisted user overrides that name a hidden mode are dropped when the active workspace cannot use them.

Kind scope lives next to `scopedDocumentIds` in the query settings slice. It is workspace-scoped in the persisted key so a graph workspace does not inherit an image filter from a normal one. Switching workspace clears kind chips if the stored workspace id differs.

## Types

`src/types/workspace.ts` `Workspace` and `CreateWorkspaceRequest` gain the three fields and `rag_profile_capabilities`. `UpdateWorkspaceRequest` does not gain them. The client function `updateWorkspace` must not forward them if a caller spreads a `Workspace` into the update body. Strip in `updateWorkspace` as a backstop ([EC-180-04](14-edge-cases.md)).

## Data loading

Workspace query is already on the layout. Nav waits for it. No new endpoint. Capabilities ride the workspace GET.

Eligible embedding models: existing `useEmbeddingModels`. Filter with `image_embedding === true` when the draft modalities are `text,image`. The flag is added to the models DTO in [08-multimodal-embedding](08-multimodal-embedding.md).

## Tests (UI)

| Test | Asserts |
|------|---------|
| `rag-profile-locales.test.ts` | en, fr, zh key parity |
| wizard unit | normal draft skips extraction steps |
| `buildWorkspaceIngestPayload` | create includes profile; update helper omits it |
| `modesForProfile` | normal returns naive and bypass only |
| Playwright | [16-e2e-test-matrix](16-e2e-test-matrix.md) `T-180-ui-wizard`, `T-180-ui-query` |

Components stay presentational. Eligibility and payload rules live in the lib functions so the tests do not render the wizard.

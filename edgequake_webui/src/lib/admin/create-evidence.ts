/**
 * SPEC-183 — the Evidence decision on the Create workspace dialog.
 *
 * SPEC-180 made `embedding_modalities` a create-only, immutable field, and the create dialog
 * (SPEC-181 W-03) asked three questions that did not include it. The field defaulted to
 * `text` for every workspace a person could create from the UI, and because the column is
 * immutable that put image uploads permanently out of reach: the server answered, correctly,
 * "this workspace embeds text only", with no control anywhere that could change it.
 *
 * The decision belongs beside the retrieval type, for the same reason the retrieval type is
 * there: both are fixed at create, so both must be asked before create.
 *
 * Pure helpers only, so the rules are unit-testable without rendering the dialog.
 */

import type { EmbeddingModalities } from '@/constants/rag-profile';

/** What the retrieval radio currently says. */
export type RetrievalChoice = 'graph' | 'normal';

export interface EvidenceChoice {
  value: EmbeddingModalities;
  /** Text and images needs a normal workspace; it is inert, not hidden, on graph. */
  disabled: boolean;
  /** Why it is inert. `null` when the choice is available. */
  reason: string | null;
}

/** SPEC-180 LAW-180-5: a graph workspace embeds text only, so images are not offered. */
export const GRAPH_EVIDENCE_REASON =
  'Graph workspaces embed text only. Choose Normal retrieval to store images.';

/**
 * The evidence a create will actually send.
 *
 * A graph workspace never carries `text,image`: the server answers
 * `modalities_require_normal`, and sending it would make the dialog fail on a combination
 * the dialog itself offered. So the choice is clamped here, in one place.
 */
export function resolveCreateModalities(
  retrieval: RetrievalChoice,
  chosen: EmbeddingModalities,
): EmbeddingModalities {
  if (retrieval === 'graph') return 'text';
  return chosen;
}

/** Both evidence options, with their availability for the current retrieval choice. */
export function evidenceChoices(retrieval: RetrievalChoice): EvidenceChoice[] {
  return [
    { value: 'text', disabled: false, reason: null },
    {
      value: 'text,image',
      disabled: retrieval === 'graph',
      reason: retrieval === 'graph' ? GRAPH_EVIDENCE_REASON : null,
    },
  ];
}

/** How the Evidence section reads when a text-only embedding model is all that is offered. */
export const NO_IMAGE_MODEL_REASON =
  'No image-capable embedding model is offered here. Ask a deployment admin to offer one (for example gemini-embedding-2 on Vertex AI), or keep Text only.';

/**
 * True when the dialog wants images but nothing it can pick can read them.
 *
 * This is deliberately about the *pinned or inherited* pair the dialog can see, because the
 * server refuses the create with `embedding_model_not_multimodal` and the operator needs the
 * reason before pressing the button, not after.
 */
export function imagesUnavailable(args: {
  retrieval: RetrievalChoice;
  modalities: EmbeddingModalities;
  imageModelOffered: boolean;
}): boolean {
  return (
    args.retrieval === 'normal' &&
    args.modalities === 'text,image' &&
    !args.imageModelOffered
  );
}

export interface ImageEmbeddingOfferLike {
  provider?: string;
  model?: string;
}

/**
 * The embedding pair an image workspace should store, or `null` to leave the draft alone.
 *
 * WHY this exists: the inherited embedding model is usually text-only, so choosing "Text and
 * images" would otherwise leave Create disabled until the operator found the Embed row and
 * picked a different model — a detour that reads as the same dead end this pack is fixing. When
 * an image-capable model is offered, choosing images selects it, and the card prints which model
 * that is so the change is visible rather than magic. Nothing happens when the pair already
 * reads images, and nothing happens when the offer has no such model: that case is a refusal,
 * not a guess.
 */
export function imageEmbeddingPin(args: {
  retrieval: RetrievalChoice;
  modalities: EmbeddingModalities;
  current: ImageEmbeddingOfferLike;
  offered: readonly { provider: string; model: string; supports_vision?: boolean }[];
  pairReadsImages: boolean;
}): { provider: string; model: string } | null {
  if (args.retrieval !== 'normal' || args.modalities !== 'text,image') return null;
  if (args.pairReadsImages) return null;
  const pick = args.offered.find((m) => m.supports_vision === true);
  if (!pick) return null;
  // Already pinned to that pair: no state change, so no needless re-render.
  if (args.current.provider === pick.provider && args.current.model === pick.model) {
    return null;
  }
  return { provider: pick.provider, model: pick.model };
}

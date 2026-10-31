/**
 * SPEC-180 — draft edits for the retrieval step (pure, unit-tested).
 *
 * Each function returns a patch. The wizard applies it, so the rules live here and not in
 * the radio handlers.
 */

import type { EmbeddingModalities, RagProfile } from '@/constants/rag-profile';
import type { WizardDraft } from '@/lib/onboarding/wizard-state';

type Source = Pick<WizardDraft, 'ragProfile' | 'embeddingModalities' | 'embeddingPick'>;

/** True when the draft embeds images, so its embedding model must accept them. */
export function draftEmbedsImages(
  draft: Pick<WizardDraft, 'ragProfile' | 'embeddingModalities'>,
): boolean {
  return draft.ragProfile === 'normal' && draft.embeddingModalities === 'text,image';
}

/** The embedding pick is dropped when it cannot serve the draft's modalities. */
function pickPatch(
  draft: Pick<WizardDraft, 'embeddingPick'>,
  modalities: EmbeddingModalities,
): Partial<WizardDraft> {
  if (modalities !== 'text,image' || draft.embeddingPick?.embedsImages === true) return {};
  return draft.embeddingPick ? { embeddingPick: undefined } : {};
}

/**
 * Choosing images forces custom mode, because the inherited text-only model cannot serve it.
 * Coming back to Text with nothing picked must undo that, or the models step says "pick at
 * least one model" with an empty form — the dead end behind an unusable image model.
 */
function backToDefaultsWhenEmpty(draft: WizardDraft): Partial<WizardDraft> {
  const anyPick = Boolean(draft.llmPick || draft.embeddingPick || draft.visionPick);
  return anyPick ? {} : { useServerDefaults: true };
}

/** Choosing a profile. Graph cannot embed images, so it resets the evidence to text. */
export function patchForProfile(draft: Source, profile: RagProfile): Partial<WizardDraft> {
  if (profile === draft.ragProfile) return {};
  if (profile === 'graph') {
    return {
      ragProfile: 'graph',
      embeddingModalities: 'text',
      ...backToDefaultsWhenEmpty({ ...draft, embeddingPick: undefined } as WizardDraft),
    };
  }
  return { ragProfile: 'normal' };
}

/** Choosing the evidence. Only a normal draft may pick images. */
export function patchForModalities(
  draft: Source,
  modalities: EmbeddingModalities,
): Partial<WizardDraft> {
  if (draft.ragProfile !== 'normal' && modalities === 'text,image') return {};
  const dropped = pickPatch(draft, modalities);
  if (modalities === 'text') {
    // Leaving images with no other pick returns the draft to inherited defaults.
    return {
      embeddingModalities: modalities,
      ...dropped,
      ...backToDefaultsWhenEmpty({ ...draft, embeddingPick: undefined } as WizardDraft),
    };
  }
  return { embeddingModalities: modalities, ...dropped };
}

interface SavedEmbedding {
  provider?: string | null;
  model?: string | null;
}

/**
 * False when the draft embeds images but the embedding pick cannot read them.
 * Reconfigure passes the workspace's saved model: keeping it is always fine, because the
 * server already accepted it for this workspace.
 */
export function imageEmbeddingOk(draft: Source, saved?: SavedEmbedding): boolean {
  if (!draftEmbedsImages(draft)) return true;
  const pick = draft.embeddingPick;
  if (pick?.embedsImages === true) return true;
  return Boolean(
    pick && saved && pick.provider === saved.provider && pick.model === saved.model,
  );
}

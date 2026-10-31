import type { EmbeddingSelection } from '@/components/workspace/embedding-model-selector';
import type { LLMSelection } from '@/components/workspace/llm-model-selector';
import type { WizardRole } from '@/lib/onboarding/defaults-eligibility';
import type { WizardDraft, WizardModelPick } from '@/lib/onboarding/wizard-state';

export function llmSelectionFromPick(
  pick?: WizardModelPick,
): LLMSelection | undefined {
  if (!pick?.provider || !pick?.model) return undefined;
  return {
    provider: pick.provider,
    model: pick.model,
    fullId: pick.fullId ?? `${pick.provider}/${pick.model}`,
  };
}

export function embeddingSelectionFromPick(
  pick?: WizardModelPick,
): EmbeddingSelection | undefined {
  if (!pick?.provider || !pick?.model) return undefined;
  return {
    provider: pick.provider,
    model: pick.model,
    // SPEC-179: never guess a vector size. 0 means "not registered"; the wizard blocks on it.
    dimension:
      typeof pick.dimension === 'number' && pick.dimension > 0 ? pick.dimension : 0,
    embedsImages: pick.embedsImages,
  };
}

export function pickFromLlm(sel?: LLMSelection): WizardModelPick | undefined {
  if (!sel?.provider || !sel?.model) return undefined;
  return {
    provider: sel.provider,
    model: sel.model,
    fullId: sel.fullId,
  };
}

export function pickFromEmbedding(
  sel?: EmbeddingSelection,
): WizardModelPick | undefined {
  if (!sel?.provider || !sel?.model) return undefined;
  return {
    provider: sel.provider,
    model: sel.model,
    dimension: sel.dimension,
    embedsImages: sel.embedsImages,
  };
}

export function draftPatchAfterLlmPick(
  draft: WizardDraft,
  next: LLMSelection | undefined,
): Partial<WizardDraft> {
  if (next?.provider && next?.model) {
    return {
      llmPick: pickFromLlm(next),
      useServerDefaults: false,
      advancedOpen: true,
    };
  }
  const remaining = Boolean(draft.embeddingPick || draft.visionPick);
  return {
    llmPick: undefined,
    useServerDefaults: remaining ? draft.useServerDefaults : true,
  };
}

export function draftPatchAfterEmbeddingPick(
  draft: WizardDraft,
  next: EmbeddingSelection | undefined,
): Partial<WizardDraft> {
  if (next?.provider && next?.model) {
    return {
      embeddingPick: pickFromEmbedding(next),
      useServerDefaults: false,
      advancedOpen: true,
    };
  }
  const remaining = Boolean(draft.llmPick || draft.visionPick);
  return {
    embeddingPick: undefined,
    useServerDefaults: remaining ? draft.useServerDefaults : true,
  };
}

export function draftPatchAfterVisionPick(
  draft: WizardDraft,
  next: LLMSelection | undefined,
): Partial<WizardDraft> {
  if (next?.provider && next?.model) {
    return {
      visionPick: pickFromLlm(next),
      useServerDefaults: false,
      advancedOpen: true,
    };
  }
  const remaining = Boolean(draft.llmPick || draft.embeddingPick);
  return {
    visionPick: undefined,
    useServerDefaults: remaining ? draft.useServerDefaults : true,
  };
}

type PickLike = { provider?: string; model?: string } | undefined;

/**
 * Custom models need at least one real pick. A role left unpicked inherits the default
 * (the pickers offer exactly that), so a partial override is valid on its own.
 */
export function hasAnyModelPick(picks: Record<WizardRole, PickLike>): boolean {
  return Object.values(picks).some((pick) => Boolean(pick?.provider && pick?.model));
}

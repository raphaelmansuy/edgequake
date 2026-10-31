'use client';

/**
 * SPEC-179 — judges the pair a wizard will save against the offer for its scope.
 * Reads `/models/eligible` (never the raw inventory), so it agrees with the pickers.
 */

import { useMemo } from 'react';
import { useChatFamilyModels, useEmbeddingSupport, useScopedEligibleModels } from '@/hooks/use-model-policy';
import {
  isBlocked,
  judgeRole,
  nothingOffered,
  type ModelPair,
  type RoleList,
  type Verdicts,
  type WizardRole,
} from '@/lib/onboarding/defaults-eligibility';
import type { ModelDefaultsSlice } from '@/lib/onboarding/inherited-defaults';

export type WizardScope = 'workspace' | 'tenant' | 'deployment';

export interface WizardEligibilityInput {
  scope: WizardScope;
  id: string | null | undefined;
  /** True when the draft inherits (no custom picks). */
  useDefaults: boolean;
  inherited: ModelDefaultsSlice;
  llm?: ModelPair;
  embedding?: ModelPair;
  vision?: ModelPair;
  /** SPEC-180 08 — probe the image embedding pair before create can refuse it. */
  probeImages?: boolean;
}

export interface WizardEligibility {
  verdicts: Verdicts;
  /** The pair each verdict judged. Shown by the notice. */
  pairs: Record<WizardRole, ModelPair>;
  blocked: boolean;
  nothingOffered: boolean;
  isLoading: boolean;
  isError: boolean;
  /**
   * SPEC-180 08 — the image embedding probe for the pair this wizard will save.
   * `failed` carries the server's own sentence, so the model step explains itself.
   */
  imageProbe: {
    status: 'idle' | 'checking' | 'ok' | 'failed';
    code?: string;
    message?: string;
  };
}

export function useWizardEligibility(input: WizardEligibilityInput): WizardEligibility {
  const chat = useChatFamilyModels({ scope: input.scope, id: input.id });
  const embeddingQuery = useScopedEligibleModels({ scope: input.scope, id: input.id, usage: 'embedding' });
  const visionQuery = useScopedEligibleModels({ scope: input.scope, id: input.id, usage: 'vlm' });
  const { useDefaults, inherited, llm, embedding, vision } = input;
  const probe = useEmbeddingSupport({
    scope: input.scope,
    id: input.id,
    provider: embedding?.provider,
    model: embedding?.model,
    dimension: embedding?.dimension,
    enabled: input.probeImages === true && Boolean(embedding?.provider && embedding?.model),
  });

  return useMemo(() => {
    const inheritedPairs: Record<WizardRole, ModelPair> = {
      llm: { provider: inherited.defaultLlmProvider, model: inherited.defaultLlmModel },
      embedding: {
        provider: inherited.defaultEmbeddingProvider,
        model: inherited.defaultEmbeddingModel,
        dimension: inherited.defaultEmbeddingDimension,
      },
      vision: { provider: inherited.defaultVisionProvider, model: inherited.defaultVisionModel },
    };
    // A role the user did not pick inherits, so the inherited pair is what gets judged.
    const pairs: Record<WizardRole, ModelPair> = useDefaults
      ? inheritedPairs
      : {
          llm: llm?.model ? llm : inheritedPairs.llm,
          embedding: embedding?.model ? embedding : inheritedPairs.embedding,
          vision: vision?.model ? vision : inheritedPairs.vision,
        };
    const lists: Record<WizardRole, RoleList> = {
      llm: { models: chat.models, loading: chat.isLoading },
      embedding: { models: embeddingQuery.data?.models ?? [], loading: embeddingQuery.isLoading },
      vision: { models: visionQuery.data?.models ?? [], loading: visionQuery.isLoading },
    };
    const verdicts: Verdicts = {
      llm: judgeRole('llm', pairs.llm, lists.llm),
      embedding: judgeRole('embedding', pairs.embedding, lists.embedding),
      vision: judgeRole('vision', pairs.vision, lists.vision),
    };
    const isError = chat.isError || embeddingQuery.isError || visionQuery.isError;
    // SPEC-180 08: only an image-embedding draft needs the probe, and only once its pair is
    // eligible — a refused pair already has a clearer reason.
    const wantsImages = input.probeImages === true;
    const imageProbe: WizardEligibility['imageProbe'] = !wantsImages
      ? { status: 'idle' }
      : probe.isFetching
        ? { status: 'checking' }
        : probe.data
          ? probe.data.embeds_images === false
            ? {
                status: 'failed',
                code: probe.data.code ?? 'embedding_auth',
                message: probe.data.message ?? undefined,
              }
            : { status: 'ok' }
          : probe.isError
            ? { status: 'failed', code: 'embedding_request_failed' }
            : { status: 'idle' };
    return {
      verdicts,
      pairs,
      blocked: isBlocked(verdicts),
      nothingOffered: !isError && nothingOffered(lists),
      isLoading: Object.values(lists).some((list) => list.loading),
      isError,
      imageProbe,
    };
  }, [
    useDefaults,
    inherited,
    llm,
    embedding,
    vision,
    chat.models,
    chat.isLoading,
    chat.isError,
    embeddingQuery.data,
    embeddingQuery.isLoading,
    embeddingQuery.isError,
    visionQuery.data,
    visionQuery.isLoading,
    visionQuery.isError,
    input.probeImages,
    probe.isFetching,
    probe.data,
    probe.isError,
  ]);
}

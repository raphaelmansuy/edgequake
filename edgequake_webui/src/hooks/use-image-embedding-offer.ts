"use client";

/**
 * SPEC-183 — which embedding models this scope is offered, and whether any reads images.
 *
 * The Create workspace dialog needs this to know whether "Text and images" is achievable at
 * all. It reads `GET /models/eligible?usage=embedding` (the same query key the eligibility
 * hook already uses), so React Query serves it from cache and no second request is made.
 */

import { useScopedEligibleModels } from "@/hooks/use-model-policy";
import type { WizardScope } from "@/hooks/use-wizard-eligibility";
import { useMemo } from "react";

export interface ImageEmbeddingOffer {
  /** False until the offer has answered, so the dialog can stay quiet instead of accusing. */
  loaded: boolean;
  imageModelOffered: boolean;
  /** The first image-capable pair, for a sentence that names the model. */
  imageModelLabel: string | null;
  /** Does this exact pair read images? Used to catch a pinned text-only model. */
  pairReadsImages: (provider?: string, model?: string) => boolean;
  /** The embedding offer, so a caller can pick an image-capable pair when images are wanted. */
  offered: { provider: string; model: string; supports_vision?: boolean }[];
}

export function useImageEmbeddingOffer(input: {
  scope: WizardScope;
  id: string | null | undefined;
}): ImageEmbeddingOffer {
  const query = useScopedEligibleModels({
    scope: input.scope,
    id: input.id,
    usage: "embedding",
  });

  return useMemo(() => {
    const models = query.data?.models ?? [];
    const imageModels = models.filter((m) => m.supports_vision === true);
    const first = imageModels[0];
    const pairReadsImages = (provider?: string, model?: string) => {
      if (!provider || !model) return false;
      return models.some(
        (m) =>
          m.provider.toLowerCase() === provider.toLowerCase() &&
          m.model.toLowerCase() === model.toLowerCase() &&
          m.supports_vision === true,
      );
    };
    return {
      loaded: !query.isLoading && query.data != null,
      imageModelOffered: imageModels.length > 0,
      imageModelLabel: first ? `${first.provider}/${first.model}` : null,
      pairReadsImages,
      offered: models.map((m) => ({
        provider: m.provider,
        model: m.model,
        supports_vision: m.supports_vision,
      })),
    };
  }, [query.data, query.isLoading]);
}

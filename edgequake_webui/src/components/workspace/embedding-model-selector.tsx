/**
 * Embedding model selector for workspace configuration (SPEC-043 two-step picker).
 */
'use client';

import { mergePickerOptions, pairInCatalog } from '@/components/models/model-picker-options';
import { ModelsStatus } from '@/components/models/models-status';
import {
  ModelPickerPanel,
  formatModelFullId,
  type ModelPickerOption,
  type ModelPickerValue,
} from '@/components/models/model-picker-panel';
import { useScopedEligibleModels } from '@/hooks/use-model-policy';
import type { EligibleModel } from '@/lib/api/edgequake/model-catalog';
import { cn } from '@/lib/utils';
import { useTenantStore } from '@/stores/use-tenant-store';
import { Brain, ImageOff } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export interface EmbeddingSelection {
  model: string;
  provider: string;
  dimension: number;
  /** SPEC-180 — the catalog says this model accepts images. */
  embedsImages?: boolean;
}

interface EmbeddingModelSelectorProps {
  value?: EmbeddingSelection;
  onChange?: (selection: EmbeddingSelection | undefined) => void;
  disabled?: boolean;
  className?: string;
  /** Layout hook for the provider + model panel (e.g. side by side in wide forms). */
  panelClassName?: string;
  /** @deprecated Ignored — provider is a dedicated select. */
  showProviderFilters?: boolean;
  /** LAW-101-13 inherit chip; overrides fleet catalog default label. */
  inheritLabel?: string;
  /** Inherited pair. The "use default" link is hidden when this pair is not eligible. */
  inheritPair?: { provider?: string; model?: string };
  choiceScope?: 'workspace' | 'tenant' | 'deployment';
  /** Explicit scope id; see `LLMModelSelector` (create-workspace targets its own tenant). */
  scopeId?: string | null;
  /** SPEC-180: list only models that accept images (a `text,image` workspace). */
  imageOnly?: boolean;
}

function toOption(model: EligibleModel): ModelPickerOption {
  return {
    provider: model.provider,
    providerDisplayName: model.provider_display_name || model.provider,
    name: model.model,
    displayName: model.display_name || model.model,
    fullId: formatModelFullId(model.provider, model.model),
    contextLength: 0,
    // For an embedding card the flag means "embeds images" (SPEC-180 08).
    supportsVision: model.supports_vision === true,
    supportsTools: false,
    supportsStreaming: false,
    dimension: model.embedding_dimension ?? undefined,
    available: true,
  };
}

export function EmbeddingModelSelector({
  value,
  onChange,
  disabled,
  className,
  panelClassName,
  inheritLabel,
  inheritPair,
  choiceScope = 'workspace',
  scopeId,
  imageOnly = false,
}: EmbeddingModelSelectorProps) {
  const { t } = useTranslation();
  const workspaceId = useTenantStore((s) => s.selectedWorkspaceId);
  const tenantId = useTenantStore((s) => s.selectedTenantId);
  const id =
    scopeId !== undefined
      ? scopeId
      : choiceScope === 'tenant'
        ? tenantId
        : choiceScope === 'workspace'
          ? workspaceId
          : null;
  const { data, isLoading, error, refetch } = useScopedEligibleModels({
    scope: choiceScope,
    id,
    usage: 'embedding',
  });

  if (isLoading) {
    return (
      <div data-testid="embedding-model-selector-loading">
        <ModelsStatus state="loading" className={className} />
      </div>
    );
  }

  if (error || !data) {
    return (
      <div data-testid="embedding-model-selector">
        <ModelsStatus state="error" onRetry={() => void refetch()} className={className} />
      </div>
    );
  }

  const offered = imageOnly ? data.models.filter((m) => m.supports_vision === true) : data.models;
  // SPEC-180 08: "Text and images" but nothing here reads images. An empty picker is a dead
  // end; say what is missing and who can fix it, and let the caller offer Text instead.
  if (imageOnly && offered.length === 0) {
    return (
      <div
        className={cn('flex items-start gap-2 rounded-lg border border-dashed p-3 text-xs', className)}
        data-testid="embedding-model-selector-none-image-capable"
        role="status"
      >
        <ImageOff aria-hidden className="mt-px h-4 w-4 shrink-0 text-muted-foreground" />
        <span className="leading-relaxed text-muted-foreground">
          {t(
            'ragProfile.noImageEmbedding',
            'No image-capable embedding model is offered here. Ask a deployment admin to offer one (for example gemini-embedding-2 on Vertex AI), or go back and choose Text instead.',
          )}
        </span>
      </div>
    );
  }
  const options = mergePickerOptions(offered.map(toOption));

  const pickerValue: ModelPickerValue | undefined = value
    ? {
        provider: value.provider,
        model: value.model,
        fullId: `${value.provider}/${value.model}`,
      }
    : undefined;

  const defaultLabel = inheritLabel ?? 'Inherit';

  const handleChange = (v: ModelPickerValue | undefined) => {
    if (!v) {
      onChange?.(undefined);
      return;
    }
    const match = data.models.find((m) => m.provider === v.provider && m.model === v.model);
    onChange?.({
      provider: v.provider,
      model: v.model,
      dimension: match?.embedding_dimension ?? 0,
      embedsImages: match?.supports_vision === true,
    });
  };

  return (
    <div className={cn('space-y-1', className)} data-testid="embedding-model-selector">
      <ModelPickerPanel
        className={panelClassName}
        variant="embedding"
        options={options}
        value={pickerValue}
        onChange={handleChange}
        disabled={disabled}
        serverDefaultLabel={defaultLabel}
        allowServerDefault={
          !inheritPair?.provider ||
          !inheritPair.model ||
          pairInCatalog(inheritPair.provider, inheritPair.model, options)
        }
        placeholder="Search embedding models…"
        testId="embedding-model-picker-panel"
      />
      <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
        <Brain className="h-3 w-3" />
        <span>
          {imageOnly
            ? 'Reads text and images into one vector space'
            : 'Used for vector embeddings of document chunks'}
        </span>
      </div>
    </div>
  );
}

export { parseModelFullId as parseFullId } from '@/components/models/model-picker-panel';

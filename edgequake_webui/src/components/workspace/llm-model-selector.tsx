/**
 * LLM model selector. Lists `eligible_models` for the scope, not the inventory.
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
import type { EligibleModel, UsageKey } from '@/lib/api/edgequake/model-catalog';
import { cn } from '@/lib/utils';
import { useTenantStore } from '@/stores/use-tenant-store';
import { Sparkles } from 'lucide-react';

export interface LLMSelection {
  model: string;
  provider: string;
  fullId: string;
}

interface LLMModelSelectorProps {
  value?: LLMSelection;
  onChange?: (selection: LLMSelection | undefined) => void;
  disabled?: boolean;
  className?: string;
  /** Layout hook for the provider + model panel (e.g. side by side in wide forms). */
  panelClassName?: string;
  showUsageHint?: boolean;
  filterVision?: boolean;
  /** @deprecated Ignored — provider is a dedicated select. */
  showProviderFilters?: boolean;
  showCapabilityFilters?: boolean;
  inheritLabel?: string;
  /** Inherited pair. The "use default" link is hidden when this pair is not eligible. */
  inheritPair?: { provider?: string; model?: string };
  /** Defaults to the workspace on screen. */
  choiceScope?: 'workspace' | 'tenant' | 'deployment';
  /**
   * Explicit scope id. Create-workspace asks the *tenant it was opened for*, which is not
   * necessarily the tenant in the store yet (a platform admin creating for someone else).
   */
  scopeId?: string | null;
  usage?: UsageKey;
}

function toOption(model: EligibleModel): ModelPickerOption {
  return {
    provider: model.provider,
    providerDisplayName: model.provider_display_name || model.provider,
    name: model.model,
    displayName: model.display_name || model.model,
    fullId: formatModelFullId(model.provider, model.model),
    contextLength: 0,
    supportsVision: model.supports_vision,
    supportsTools: false,
    supportsStreaming: model.supports_streaming !== false,
    available: true,
  };
}

export function LLMModelSelector({
  value,
  onChange,
  disabled,
  className,
  panelClassName,
  showUsageHint = true,
  filterVision = false,
  showCapabilityFilters,
  inheritLabel,
  inheritPair,
  choiceScope = 'workspace',
  scopeId,
  usage,
}: LLMModelSelectorProps) {
  const workspaceId = useTenantStore((s) => s.selectedWorkspaceId);
  const tenantId = useTenantStore((s) => s.selectedTenantId);
  const resolvedUsage: UsageKey = usage ?? (filterVision ? 'vlm' : 'query');
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
    usage: resolvedUsage,
  });

  if (isLoading) {
    return <ModelsStatus state="loading" className={className} />;
  }

  if (error || !data) {
    return <ModelsStatus state="error" onRetry={() => void refetch()} className={className} />;
  }

  const models = filterVision ? data.models.filter((model) => model.supports_vision) : data.models;
  const options = mergePickerOptions(models.map(toOption));
  const pickerValue: ModelPickerValue | undefined = value
    ? { provider: value.provider, model: value.model, fullId: value.fullId }
    : undefined;

  return (
    <div className={cn('space-y-1', className)} data-testid="llm-model-selector">
      <ModelPickerPanel
        className={panelClassName}
        options={options}
        value={pickerValue}
        onChange={(next) =>
          onChange?.(
            next ? { provider: next.provider, model: next.model, fullId: next.fullId } : undefined,
          )
        }
        disabled={disabled}
        filterVision={filterVision}
        showCapabilityFilters={
          showCapabilityFilters !== undefined ? showCapabilityFilters : !filterVision
        }
        serverDefaultLabel={inheritLabel ?? 'Inherit'}
        allowServerDefault={
          !inheritPair?.provider ||
          !inheritPair.model ||
          pairInCatalog(inheritPair.provider, inheritPair.model, options)
        }
        placeholder="Search models…"
        enableRemoteSearch={false}
      />
      {showUsageHint && (
        <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <Sparkles className="h-3 w-3" />
          <span>
            {filterVision
              ? 'Used for PDF-to-Markdown image extraction (requires vision capability)'
              : 'Models this scope allows for this job.'}
          </span>
        </div>
      )}
    </div>
  );
}

export {
  formatModelFullId as formatFullId,
  parseModelFullId as parseFullId,
} from '@/components/models/model-picker-panel';

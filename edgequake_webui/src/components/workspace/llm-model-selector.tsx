/**
 * LLM model selector for workspace configuration (SPEC-043 unified picker).
 */
'use client';

import { llmModelToPickerOption } from '@/components/models/model-picker-mappers';
import { mergePickerOptions } from '@/components/models/model-picker-options';
import {
  ModelPickerPanel,
  type ModelPickerValue,
} from '@/components/models/model-picker-panel';
import { useLlmModels } from '@/hooks/use-providers';
import { cn } from '@/lib/utils';
import { Loader2, Sparkles } from 'lucide-react';

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
  showUsageHint?: boolean;
  filterVision?: boolean;
}

export function LLMModelSelector({
  value,
  onChange,
  disabled,
  className,
  showUsageHint = true,
  filterVision = false,
}: LLMModelSelectorProps) {
  const { data: llmData, isLoading, error } = useLlmModels();

  if (isLoading) {
    return (
      <div className={cn('flex items-center gap-2 px-3 py-2 bg-muted rounded-lg', className)}>
        <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
        <span className="text-sm text-muted-foreground">Loading LLM models...</span>
      </div>
    );
  }

  if (error || !llmData) {
    return (
      <div className={cn('px-3 py-2 bg-muted rounded-lg text-sm text-muted-foreground', className)}>
        Could not load LLM models. Will use server default.
      </div>
    );
  }

  const filtered = filterVision
    ? llmData.models.filter((m) => m.capabilities.supports_vision)
    : llmData.models;

  const options = mergePickerOptions(filtered.map(llmModelToPickerOption));

  const pickerValue: ModelPickerValue | undefined = value
    ? { provider: value.provider, model: value.model, fullId: value.fullId }
    : undefined;

  const defaultLabel =
    llmData.default_provider && llmData.default_model
      ? `Server default (${llmData.default_provider}/${llmData.default_model})`
      : 'Server default';

  return (
    <div className={cn('space-y-1', className)} data-testid="llm-model-selector">
      <ModelPickerPanel
        options={options}
        value={pickerValue}
        onChange={(v) =>
          onChange?.(
            v ? { provider: v.provider, model: v.model, fullId: v.fullId } : undefined,
          )
        }
        disabled={disabled}
        filterVision={filterVision}
        showCapabilityFilters={!filterVision}
        serverDefaultLabel={defaultLabel}
        placeholder="Search LLM models…"
      />
      {showUsageHint && (
        <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <Sparkles className="h-3 w-3" />
          <span>
            {filterVision
              ? 'Used for PDF-to-Markdown image extraction (requires vision capability)'
              : 'Used for document ingestion, entity extraction, and summarization'}
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

/**
 * Embedding model selector for workspace configuration (SPEC-043 unified picker).
 */
'use client';

import { embeddingModelToPickerOption } from '@/components/models/model-picker-mappers';
import { mergePickerOptions } from '@/components/models/model-picker-options';
import {
  ModelPickerPanel,
  type ModelPickerValue,
  parseModelFullId,
} from '@/components/models/model-picker-panel';
import { useEmbeddingModels } from '@/hooks/use-providers';
import { cn } from '@/lib/utils';
import { Brain, Loader2 } from 'lucide-react';

export interface EmbeddingSelection {
  model: string;
  provider: string;
  dimension: number;
}

interface EmbeddingModelSelectorProps {
  value?: EmbeddingSelection;
  onChange?: (selection: EmbeddingSelection | undefined) => void;
  disabled?: boolean;
  className?: string;
}

export function EmbeddingModelSelector({
  value,
  onChange,
  disabled,
  className,
}: EmbeddingModelSelectorProps) {
  const { data: embeddingData, isLoading, error } = useEmbeddingModels();

  if (isLoading) {
    return (
      <div
        className={cn('flex items-center gap-2 px-3 py-2 bg-muted rounded-lg', className)}
        data-testid="embedding-model-selector-loading"
      >
        <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
        <span className="text-sm text-muted-foreground">Loading embedding models...</span>
      </div>
    );
  }

  if (error || !embeddingData) {
    return (
      <div
        className={cn('px-3 py-2 bg-muted rounded-lg text-sm text-muted-foreground', className)}
        data-testid="embedding-model-selector"
      >
        Could not load embedding models. Will use server default.
      </div>
    );
  }

  const options = mergePickerOptions(embeddingData.models.map(embeddingModelToPickerOption));

  const pickerValue: ModelPickerValue | undefined = value
    ? {
        provider: value.provider,
        model: value.model,
        fullId: `${value.provider}/${value.model}`,
      }
    : undefined;

  const defaultLabel =
    embeddingData.default_provider && embeddingData.default_model
      ? `Server default (${embeddingData.default_provider}/${embeddingData.default_model})`
      : 'Server default';

  const handleChange = (v: ModelPickerValue | undefined) => {
    if (!v) {
      onChange?.(undefined);
      return;
    }
    const match = embeddingData.models.find(
      (m) => m.provider === v.provider && m.name === v.model,
    );
    if (match) {
      onChange?.({
        provider: v.provider,
        model: v.model,
        dimension: match.dimension,
      });
    }
  };

  return (
    <div className={cn('space-y-1', className)} data-testid="embedding-model-selector">
      <ModelPickerPanel
        variant="embedding"
        options={options}
        value={pickerValue}
        onChange={handleChange}
        disabled={disabled}
        serverDefaultLabel={defaultLabel}
        placeholder="Search embedding models…"
        testId="embedding-model-picker-panel"
      />
      <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
        <Brain className="h-3 w-3" />
        <span>Used for vector embeddings of document chunks</span>
      </div>
    </div>
  );
}

export { parseModelFullId as parseFullId } from '@/components/models/model-picker-panel';

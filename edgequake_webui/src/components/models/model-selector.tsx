/**
 * Legacy ModelSelector facade — delegates to SPEC-043 unified pickers.
 * Value format: `provider:model` (colon) for tenant/workspace APIs.
 */
'use client';

import {
  colonValueToFullId,
  fullIdToColonValue,
} from '@/components/models/model-picker-mappers';
import { parseModelFullId } from '@/components/models/model-picker-panel';
import {
  EmbeddingModelSelector,
  type EmbeddingSelection,
} from '@/components/workspace/embedding-model-selector';
import {
  LLMModelSelector,
  type LLMSelection,
} from '@/components/workspace/llm-model-selector';
import { cn } from '@/lib/utils';

/** @deprecated Use LLMSelection / EmbeddingSelection from workspace selectors. */
export interface DisplayModelItem {
  value: string;
  provider: string;
  providerDisplayName: string;
  name: string;
  displayName: string;
}

interface ModelSelectorProps {
  value?: string;
  onChange?: (value: string, model?: DisplayModelItem) => void;
  type: 'llm' | 'embedding';
  disabled?: boolean;
  placeholder?: string;
  className?: string;
  filterVision?: boolean;
}

function colonToLlmSelection(value: string): LLMSelection {
  const fullId = colonValueToFullId(value);
  const { provider, model } = parseModelFullId(fullId);
  return { provider, model, fullId };
}

function colonToEmbeddingSelection(value: string): EmbeddingSelection {
  const fullId = colonValueToFullId(value);
  const { provider, model } = parseModelFullId(fullId);
  return { provider, model, dimension: 0 };
}

export function ModelSelector({
  value,
  onChange,
  type,
  disabled,
  className,
  filterVision = false,
}: ModelSelectorProps) {
  if (type === 'llm') {
    const llmValue = value ? colonToLlmSelection(value) : undefined;
    return (
      <LLMModelSelector
        value={llmValue}
        onChange={(s) =>
          onChange?.(s ? fullIdToColonValue(s.fullId) : '', undefined)
        }
        filterVision={filterVision}
        showUsageHint={false}
        disabled={disabled}
        className={className}
      />
    );
  }

  const embValue = value ? colonToEmbeddingSelection(value) : undefined;
  return (
    <EmbeddingModelSelector
      value={embValue}
      onChange={(s) =>
        onChange?.(s ? fullIdToColonValue(`${s.provider}/${s.model}`) : '', undefined)
      }
      disabled={disabled}
      className={className}
    />
  );
}

export function LlmModelSelector(
  props: Omit<ModelSelectorProps, 'type' | 'placeholder'>,
) {
  return <ModelSelector {...props} type="llm" />;
}

export function EmbeddingModelSelector2(
  props: Omit<ModelSelectorProps, 'type' | 'placeholder'>,
) {
  return <ModelSelector {...props} type="embedding" />;
}

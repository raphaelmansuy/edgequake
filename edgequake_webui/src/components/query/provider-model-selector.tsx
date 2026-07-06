/**
 * Query interface model selector (SPEC-043 unified picker).
 * Value format: `provider/model` or empty string for server default.
 */
'use client';

import { LLMModelSelector, type LLMSelection } from '@/components/workspace/llm-model-selector';
import { parseModelFullId } from '@/components/models/model-picker-panel';
import { cn } from '@/lib/utils';

interface ProviderModelSelectorProps {
  value?: string;
  onChange?: (fullModelId: string) => void;
  disabled?: boolean;
  className?: string;
}

function fullIdToSelection(fullId: string): LLMSelection {
  const { provider, model } = parseModelFullId(fullId);
  return { provider, model, fullId };
}

export function ProviderModelSelector({
  value,
  onChange,
  disabled,
  className,
}: ProviderModelSelectorProps) {
  const selection = value ? fullIdToSelection(value) : undefined;

  return (
    <div className={cn(className)} data-testid="query-model-selector">
      <LLMModelSelector
        value={selection}
        onChange={(s) => onChange?.(s?.fullId ?? '')}
        disabled={disabled}
        showUsageHint={false}
      />
    </div>
  );
}

export { formatModelFullId as formatFullId, parseModelFullId as parseFullId } from '@/components/models/model-picker-panel';

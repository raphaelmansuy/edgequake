/**
 * @module EmbeddingModelSelector
 * @description Dropdown selector for choosing embedding model when creating a workspace.
 * Displays ALL available embedding models per provider with their dimensions.
 *
 * @implements SPEC-032: Ollama/LM Studio provider support - Workspace embedding selection
 * @iteration OODA #19-20 - Workspace embedding UI
 * @iteration OODA #54 - Multi-model support per provider
 *
 * @enforces BR0303 - Embedding model must be chosen at workspace creation
 * @enforces BR0304 - Dimension is auto-detected from model selection
 */
'use client';

import {
    Select,
    SelectContent,
    SelectGroup,
    SelectItem,
    SelectLabel,
    SelectTrigger,
    SelectValue,
} from '@/components/ui/select';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { useEmbeddingModels } from '@/hooks/use-providers';
import { cn } from '@/lib/utils';
import { Brain, Cloud, Cpu, FlaskConical, HelpCircle, Loader2 } from 'lucide-react';

export interface EmbeddingSelection {
  model: string;
  provider: string;
  dimension: number;
}

interface EmbeddingModelSelectorProps {
  /** Currently selected embedding model */
  value?: EmbeddingSelection;
  /** Callback when embedding selection changes */
  onChange?: (selection: EmbeddingSelection | undefined) => void;
  /** Whether the selector is disabled */
  disabled?: boolean;
  /** Additional CSS classes */
  className?: string;
}

/**
 * Get icon component for a provider.
 */
function getProviderIcon(providerId: string) {
  switch (providerId.toLowerCase()) {
    case 'openai':
      return <Cloud className="h-4 w-4 text-green-600" />;
    case 'ollama':
      return <Cpu className="h-4 w-4 text-blue-600" />;
    case 'lmstudio':
      return <Brain className="h-4 w-4 text-purple-600" />;
    case 'mock':
      return <FlaskConical className="h-4 w-4 text-gray-500" />;
    default:
      return <Brain className="h-4 w-4 text-muted-foreground" />;
  }
}

/**
 * Embedding model selector component for workspace creation.
 * Allows users to select which embedding model to use for a new workspace.
 * Shows ALL models per provider, not just defaults.
 */
export function EmbeddingModelSelector({
  value,
  onChange,
  disabled,
  className,
}: EmbeddingModelSelectorProps) {
  const { data: embeddingData, isLoading, error } = useEmbeddingModels();

  if (isLoading) {
    return (
      <div className={cn('flex items-center gap-2 px-3 py-2 bg-muted rounded-lg', className)}>
        <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
        <span className="text-sm text-muted-foreground">Loading embedding models...</span>
      </div>
    );
  }

  if (error || !embeddingData) {
    return (
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <div className={cn('flex items-center gap-2 px-3 py-2 bg-muted rounded-lg cursor-help', className)}>
              <HelpCircle className="h-4 w-4 text-muted-foreground" />
              <span className="text-sm text-muted-foreground">Using server default</span>
            </div>
          </TooltipTrigger>
          <TooltipContent>
            <p>Could not load embedding models. Will use server default.</p>
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>
    );
  }

  // Group models by provider
  const modelsByProvider = embeddingData.models.reduce((acc, model) => {
    if (!acc[model.provider]) {
      acc[model.provider] = {
        displayName: model.provider_display_name,
        models: [],
      };
    }
    acc[model.provider].models.push(model);
    return acc;
  }, {} as Record<string, { displayName: string; models: typeof embeddingData.models }>);

  // Current selection value string (provider:model format)
  const currentValue = value
    ? `${value.provider}:${value.model}`
    : undefined;

  const handleChange = (selectedId: string) => {
    if (selectedId === 'default') {
      onChange?.(undefined);
      return;
    }

    // Parse provider:model format
    const colonIdx = selectedId.indexOf(':');
    if (colonIdx === -1) return;
    const provider = selectedId.slice(0, colonIdx);
    const modelName = selectedId.slice(colonIdx + 1);

    // Find the model to get dimension
    const modelInfo = embeddingData.models.find(
      (m) => m.provider === provider && m.name === modelName
    );
    if (modelInfo) {
      onChange?.({
        model: modelName,
        provider,
        dimension: modelInfo.dimension,
      });
    }
  };

  return (
    <Select
      value={currentValue || 'default'}
      onValueChange={handleChange}
      disabled={disabled || embeddingData.models.length === 0}
    >
      <SelectTrigger className={cn('w-full', className)}>
        <SelectValue placeholder="Server default">
          {currentValue ? (
            <div className="flex items-center gap-2">
              {getProviderIcon(value?.provider || '')}
              <span className="text-sm truncate">{value?.model}</span>
              <span className="text-xs text-muted-foreground">({value?.dimension}d)</span>
            </div>
          ) : (
            <span className="text-sm text-muted-foreground">Server default</span>
          )}
        </SelectValue>
      </SelectTrigger>
      <SelectContent className="max-h-[400px]">
        {/* Default option - uses server configuration */}
        <SelectItem value="default">
          <div className="flex items-center gap-2">
            <HelpCircle className="h-4 w-4 text-muted-foreground" />
            <div className="flex flex-col">
              <span className="text-sm">Server Default</span>
              <span className="text-xs text-muted-foreground">
                {embeddingData.default_provider}/{embeddingData.default_model}
              </span>
            </div>
          </div>
        </SelectItem>

        {/* All embedding models grouped by provider */}
        {Object.entries(modelsByProvider).map(([providerId, { displayName, models }]) => (
          <SelectGroup key={providerId}>
            <SelectLabel className="text-xs font-semibold uppercase tracking-wide text-muted-foreground px-2 flex items-center gap-1">
              {getProviderIcon(providerId)}
              {displayName}
            </SelectLabel>
            {models.map((model) => {
              const selectId = `${providerId}:${model.name}`;
              return (
                <SelectItem 
                  key={selectId} 
                  value={selectId}
                  disabled={model.deprecated}
                >
                  <div className="flex items-center gap-2 w-full">
                    <div className="flex flex-col flex-1 min-w-0">
                      <div className="flex items-center gap-1.5">
                        <span className="text-sm font-medium truncate">{model.display_name}</span>
                      </div>
                      <span className="text-xs text-muted-foreground truncate">
                        {model.name} · {model.dimension}d
                      </span>
                    </div>
                  </div>
                </SelectItem>
              );
            })}
          </SelectGroup>
        ))}
      </SelectContent>
    </Select>
  );
}

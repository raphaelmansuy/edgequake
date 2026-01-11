/**
 * @module EmbeddingModelSelector
 * @description Dropdown selector for choosing embedding model when creating a workspace.
 * Displays available embedding providers with their models and dimensions.
 *
 * @implements SPEC-032: Ollama/LM Studio provider support - Workspace embedding selection
 * @iteration OODA #19-20 - Workspace embedding UI
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
import { useAvailableProviders } from '@/hooks/use-providers';
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
 */
export function EmbeddingModelSelector({
  value,
  onChange,
  disabled,
  className,
}: EmbeddingModelSelectorProps) {
  const { data: providers, isLoading, error } = useAvailableProviders();

  if (isLoading) {
    return (
      <div className={cn('flex items-center gap-2 px-3 py-2 bg-muted rounded-lg', className)}>
        <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
        <span className="text-sm text-muted-foreground">Loading models...</span>
      </div>
    );
  }

  if (error || !providers) {
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

  // Get available embedding providers
  const availableProviders = providers.embedding_providers.filter((p) => p.available);

  // Build selection options: one per embedding provider
  const options = availableProviders.map((provider) => ({
    id: `${provider.id}:${provider.default_models.embedding_model}`,
    providerId: provider.id,
    providerName: provider.name,
    model: provider.default_models.embedding_model,
    dimension: provider.default_models.embedding_dimension,
  }));

  // Current selection value string (provider:model format)
  const currentValue = value
    ? `${value.provider}:${value.model}`
    : undefined;

  const handleChange = (selectedId: string) => {
    if (selectedId === 'default') {
      onChange?.(undefined);
      return;
    }
    
    const option = options.find((o) => o.id === selectedId);
    if (option) {
      onChange?.({
        model: option.model,
        provider: option.providerId,
        dimension: option.dimension,
      });
    }
  };

  return (
    <Select
      value={currentValue || 'default'}
      onValueChange={handleChange}
      disabled={disabled || options.length === 0}
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
      <SelectContent>
        {/* Default option - uses server configuration */}
        <SelectItem value="default">
          <div className="flex items-center gap-2">
            <HelpCircle className="h-4 w-4 text-muted-foreground" />
            <div className="flex flex-col">
              <span className="text-sm">Server Default</span>
              <span className="text-xs text-muted-foreground">
                Uses server embedding configuration
              </span>
            </div>
          </div>
        </SelectItem>

        {/* Available embedding models grouped by provider */}
        {availableProviders.length > 0 && (
          <SelectGroup>
            <SelectLabel className="text-xs font-semibold uppercase tracking-wide text-muted-foreground px-2">
              Embedding Models
            </SelectLabel>
            {options.map((option) => (
              <SelectItem key={option.id} value={option.id}>
                <div className="flex items-center gap-2">
                  {getProviderIcon(option.providerId)}
                  <div className="flex flex-col">
                    <span className="text-sm font-medium">{option.providerName}</span>
                    <span className="text-xs text-muted-foreground">
                      {option.model} ({option.dimension}d)
                    </span>
                  </div>
                </div>
              </SelectItem>
            ))}
          </SelectGroup>
        )}

        {/* Show unavailable providers as disabled */}
        {providers.embedding_providers.filter((p) => !p.available).length > 0 && (
          <SelectGroup>
            <SelectLabel className="text-xs font-semibold uppercase tracking-wide text-muted-foreground px-2">
              Not Configured
            </SelectLabel>
            {providers.embedding_providers
              .filter((p) => !p.available)
              .map((provider) => (
                <SelectItem key={provider.id} value={`disabled-${provider.id}`} disabled>
                  <div className="flex items-center gap-2 opacity-50">
                    {getProviderIcon(provider.id)}
                    <span className="text-sm">{provider.name}</span>
                  </div>
                </SelectItem>
              ))}
          </SelectGroup>
        )}
      </SelectContent>
    </Select>
  );
}

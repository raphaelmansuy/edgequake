/**
 * @module LLMModelSelector
 * @description Dropdown selector for choosing LLM model when creating a workspace.
 * Displays available LLM providers with their default models and capabilities.
 *
 * @implements SPEC-032: Ollama/LM Studio provider support - Workspace LLM selection
 * @iteration OODA #10 - Workspace LLM configuration UI
 *
 * @enforces BR0305 - LLM model must be chosen at workspace creation for ingestion tasks
 * @enforces BR0306 - LLM provider is separate from query-time LLM
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
import { Brain, Cloud, Cpu, FlaskConical, HelpCircle, Loader2, Sparkles } from 'lucide-react';

export interface LLMSelection {
  /** The model name (e.g., "gemma3:12b") */
  model: string;
  /** The provider name (e.g., "ollama") */
  provider: string;
  /** Combined ID in format "provider/model" (e.g., "ollama/gemma3:12b") */
  fullId: string;
}

interface LLMModelSelectorProps {
  /** Currently selected LLM model */
  value?: LLMSelection;
  /** Callback when LLM selection changes */
  onChange?: (selection: LLMSelection | undefined) => void;
  /** Whether the selector is disabled */
  disabled?: boolean;
  /** Additional CSS classes */
  className?: string;
  /** Show additional context (used for) */
  showUsageHint?: boolean;
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
 * Format provider/model as full ID.
 */
function formatFullId(provider: string, model: string): string {
  return `${provider}/${model}`;
}

/**
 * Parse full ID into provider and model.
 */
function parseFullId(fullId: string): { provider: string; model: string } {
  const slashIndex = fullId.indexOf('/');
  if (slashIndex === -1) {
    return { provider: 'unknown', model: fullId };
  }
  return {
    provider: fullId.substring(0, slashIndex),
    model: fullId.substring(slashIndex + 1),
  };
}

/**
 * LLM model selector component for workspace creation.
 * Allows users to select which LLM to use for ingestion tasks (entity extraction, summarization).
 *
 * Note: This LLM is used for document ingestion, not for query-time chat.
 * Query-time LLM can be selected separately in the chat interface.
 */
export function LLMModelSelector({
  value,
  onChange,
  disabled,
  className,
  showUsageHint = true,
}: LLMModelSelectorProps) {
  const { data: providers, isLoading, error } = useAvailableProviders();

  if (isLoading) {
    return (
      <div className={cn('flex items-center gap-2 px-3 py-2 bg-muted rounded-lg', className)}>
        <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
        <span className="text-sm text-muted-foreground">Loading LLM providers...</span>
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
            <p>Could not load LLM providers. Will use server default.</p>
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>
    );
  }

  // Get available LLM providers
  const availableProviders = providers.llm_providers.filter((p) => p.available);

  // Build selection options: one per LLM provider
  const options = availableProviders.map((provider) => ({
    id: formatFullId(provider.id, provider.default_models.chat_model),
    providerId: provider.id,
    providerName: provider.name,
    model: provider.default_models.chat_model,
    description: provider.description,
  }));

  // Current selection value string (provider/model format)
  const currentValue = value?.fullId;

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
        fullId: option.id,
      });
    }
  };

  return (
    <div className={cn('space-y-1', className)}>
      <Select
        value={currentValue || 'default'}
        onValueChange={handleChange}
        disabled={disabled || options.length === 0}
      >
        <SelectTrigger className="w-full">
          <SelectValue placeholder="Server default">
            {currentValue ? (
              <div className="flex items-center gap-2">
                {getProviderIcon(value?.provider || '')}
                <span className="text-sm truncate">{value?.model}</span>
                <span className="text-xs text-muted-foreground capitalize">
                  ({value?.provider})
                </span>
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
                  Uses server LLM configuration
                </span>
              </div>
            </div>
          </SelectItem>

          {/* Available LLM models grouped by provider */}
          {availableProviders.length > 0 && (
            <SelectGroup>
              <SelectLabel className="text-xs font-semibold uppercase tracking-wide text-muted-foreground px-2">
                LLM Models
              </SelectLabel>
              {options.map((option) => (
                <SelectItem key={option.id} value={option.id}>
                  <div className="flex items-center gap-2">
                    {getProviderIcon(option.providerId)}
                    <div className="flex flex-col">
                      <span className="text-sm font-medium">{option.providerName}</span>
                      <span className="text-xs text-muted-foreground">
                        {option.model}
                      </span>
                    </div>
                  </div>
                </SelectItem>
              ))}
            </SelectGroup>
          )}

          {/* Show unavailable providers as disabled */}
          {providers.llm_providers.filter((p) => !p.available).length > 0 && (
            <SelectGroup>
              <SelectLabel className="text-xs font-semibold uppercase tracking-wide text-muted-foreground px-2">
                Not Configured
              </SelectLabel>
              {providers.llm_providers
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

      {/* Usage hint */}
      {showUsageHint && (
        <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <Sparkles className="h-3 w-3" />
          <span>Used for document ingestion, entity extraction, and summarization</span>
        </div>
      )}
    </div>
  );
}

export { formatFullId, parseFullId };


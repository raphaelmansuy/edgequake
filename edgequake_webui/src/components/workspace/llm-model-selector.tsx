/**
 * @module LLMModelSelector
 * @description Dropdown selector for choosing LLM model when creating a workspace.
 * Displays available LLM providers with ALL their models and capabilities.
 *
 * @implements SPEC-032: Ollama/LM Studio provider support - Workspace LLM selection
 * @iteration OODA #10 - Workspace LLM configuration UI
 * @iteration OODA #54 - Multi-model support per provider (Focus 7)
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
import { useLlmModels } from '@/hooks/use-providers';
import { cn } from '@/lib/utils';
import { Brain, Cloud, Cpu, FlaskConical, HelpCircle, Loader2, Sparkles, Eye, Zap } from 'lucide-react';

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
 * Shows ALL models per provider, not just defaults.
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
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <div className={cn('flex items-center gap-2 px-3 py-2 bg-muted rounded-lg cursor-help', className)}>
              <HelpCircle className="h-4 w-4 text-muted-foreground" />
              <span className="text-sm text-muted-foreground">Using server default</span>
            </div>
          </TooltipTrigger>
          <TooltipContent>
            <p>Could not load LLM models. Will use server default.</p>
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>
    );
  }

  // Group models by provider
  const modelsByProvider = llmData.models.reduce((acc, model) => {
    if (!acc[model.provider]) {
      acc[model.provider] = {
        displayName: model.provider_display_name,
        models: [],
      };
    }
    acc[model.provider].models.push(model);
    return acc;
  }, {} as Record<string, { displayName: string; models: typeof llmData.models }>);

  // Current selection value string (provider/model format)
  const currentValue = value?.fullId;

  const handleChange = (selectedId: string) => {
    if (selectedId === 'default') {
      onChange?.(undefined);
      return;
    }

    const { provider, model } = parseFullId(selectedId);
    onChange?.({
      model,
      provider,
      fullId: selectedId,
    });
  };

  return (
    <div className={cn('space-y-1', className)}>
      <Select
        value={currentValue || 'default'}
        onValueChange={handleChange}
        disabled={disabled || llmData.models.length === 0}
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
        <SelectContent className="max-h-[400px]">
          {/* Default option - uses server configuration */}
          <SelectItem value="default">
            <div className="flex items-center gap-2">
              <HelpCircle className="h-4 w-4 text-muted-foreground" />
              <div className="flex flex-col">
                <span className="text-sm">Server Default</span>
                <span className="text-xs text-muted-foreground">
                  {llmData.default_provider}/{llmData.default_model}
                </span>
              </div>
            </div>
          </SelectItem>

          {/* All LLM models grouped by provider */}
          {Object.entries(modelsByProvider).map(([providerId, { displayName, models }]) => (
            <SelectGroup key={providerId}>
              <SelectLabel className="text-xs font-semibold uppercase tracking-wide text-muted-foreground px-2 flex items-center gap-1">
                {getProviderIcon(providerId)}
                {displayName}
              </SelectLabel>
              {models.map((model) => {
                const fullId = formatFullId(providerId, model.name);
                return (
                  <SelectItem 
                    key={fullId} 
                    value={fullId}
                    disabled={model.deprecated}
                  >
                    <div className="flex items-center gap-2 w-full">
                      <div className="flex flex-col flex-1 min-w-0">
                        <div className="flex items-center gap-1.5">
                          <span className="text-sm font-medium truncate">{model.display_name}</span>
                          {model.capabilities.supports_vision && (
                            <span title="Vision support">
                              <Eye className="h-3 w-3 text-blue-500 flex-shrink-0" />
                            </span>
                          )}
                          {model.capabilities.supports_streaming && (
                            <span title="Streaming">
                              <Zap className="h-3 w-3 text-yellow-500 flex-shrink-0" />
                            </span>
                          )}
                        </div>
                        <span className="text-xs text-muted-foreground truncate">
                          {model.name} · {(model.capabilities.context_length / 1000).toFixed(0)}K ctx
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


/**
 * @module ProviderModelSelector
 * @description Searchable dropdown selector for choosing LLM provider and model.
 * Displays ALL available models from models.toml grouped by provider with search capability.
 * 
 * @implements SPEC-032: Ollama/LM Studio provider support - Query interface selector
 * @iteration OODA #17-18 - WebUI provider selector
 * @iteration OODA #168 - Full model selection support
 * @iteration OODA #8 - Added search/filter functionality for UX improvement
 * 
 * @enforces BR0301 - Selected provider must be available/configured
 * @enforces BR0302 - Model selection persists across sessions
 */
'use client';

import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/components/ui/command';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { useLlmModels, useAvailableProviders } from '@/hooks/use-providers';
import { cn } from '@/lib/utils';
import { Brain, Check, ChevronDown, Cloud, Cpu, Eye, FlaskConical, Loader2, Zap } from 'lucide-react';
import { useMemo, useState } from 'react';

interface ProviderModelSelectorProps {
  /** Currently selected full model ID (e.g., "ollama/gemma3:12b") */
  value?: string;
  /** Callback when model selection changes. Receives full ID "provider/model" */
  onChange?: (fullModelId: string) => void;
  /** Whether the selector is disabled */
  disabled?: boolean;
  /** Additional CSS classes */
  className?: string;
}

/**
 * Format provider and model into full ID.
 */
function formatFullId(provider: string, model: string): string {
  return `${provider}/${model}`;
}

/**
 * Parse full ID into provider and model.
 */
function parseFullId(fullId: string): { provider: string; model: string } {
  const parts = fullId.split('/');
  if (parts.length >= 2) {
    return { provider: parts[0], model: parts.slice(1).join('/') };
  }
  return { provider: fullId, model: '' };
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
 * Provider & Model selector component for query interface.
 * Shows ALL models from models.toml grouped by provider.
 * Allows users to select any specific model for queries.
 * Features searchable dropdown for easy model discovery.
 */
export function ProviderModelSelector({
  value,
  onChange,
  disabled,
  className,
}: ProviderModelSelectorProps) {
  const [open, setOpen] = useState(false);
  const { data: llmData, isLoading, error } = useLlmModels();
  const { data: providers } = useAvailableProviders();

  // Group models by provider for the dropdown
  const modelsByProvider = useMemo(() => {
    if (!llmData?.models) return {};
    
    return llmData.models.reduce((acc, model) => {
      const providerId = model.provider;
      if (!acc[providerId]) {
        acc[providerId] = {
          displayName: model.provider_display_name,
          models: [],
        };
      }
      acc[providerId].models.push(model);
      return acc;
    }, {} as Record<string, { displayName: string; models: typeof llmData.models }>);
  }, [llmData]);

  // Get available provider IDs for filtering
  const availableProviderIds = useMemo(() => {
    if (!providers?.llm_providers) return new Set<string>();
    return new Set(
      providers.llm_providers
        .filter((p) => p.available)
        .map((p) => p.id)
    );
  }, [providers]);

  // Find current selection display info
  const currentSelection = useMemo(() => {
    if (!value || !llmData?.models) return null;
    const { provider, model } = parseFullId(value);
    const modelInfo = llmData.models.find(
      (m) => m.provider === provider && m.name === model
    );
    return modelInfo
      ? { provider, model, displayName: modelInfo.display_name }
      : { provider, model, displayName: model };
  }, [value, llmData]);

  if (isLoading) {
    return (
      <div className={cn('flex items-center gap-2 px-3 py-1.5 bg-muted rounded-lg', className)}>
        <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
        <span className="text-sm text-muted-foreground">Loading models...</span>
      </div>
    );
  }

  if (error || !llmData?.models || llmData.models.length === 0) {
    return (
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <div className={cn('flex items-center gap-2 px-3 py-1.5 bg-muted rounded-lg cursor-help', className)}>
              <Brain className="h-4 w-4 text-muted-foreground" />
              <span className="text-sm text-muted-foreground">Default</span>
            </div>
          </TooltipTrigger>
          <TooltipContent>
            <p>Could not load models. Using server default.</p>
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>
    );
  }

  // Use special value for "server default" since empty string is not allowed
  const SERVER_DEFAULT = '__server_default__';

  const handleSelect = (selectedValue: string) => {
    onChange?.(selectedValue === SERVER_DEFAULT ? '' : selectedValue);
    setOpen(false);
  };

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <button
          type="button"
          role="combobox"
          aria-expanded={open}
          disabled={disabled}
          className={cn(
            'flex h-9 w-[220px] items-center justify-between rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background',
            'placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2',
            'disabled:cursor-not-allowed disabled:opacity-50',
            className
          )}
        >
          {currentSelection ? (
            <div className="flex items-center gap-2 overflow-hidden">
              {getProviderIcon(currentSelection.provider)}
              <span className="truncate">{currentSelection.displayName}</span>
            </div>
          ) : (
            <div className="flex items-center gap-2">
              <Brain className="h-4 w-4 text-muted-foreground" />
              <span className="text-muted-foreground">Server Default</span>
            </div>
          )}
          <ChevronDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
        </button>
      </PopoverTrigger>
      <PopoverContent className="w-[320px] p-0" align="start">
        <Command>
          <CommandInput placeholder="Search models..." />
          <CommandList className="max-h-[350px]">
            <CommandEmpty>No models found.</CommandEmpty>
            
            {/* Server Default option */}
            <CommandGroup heading="Default">
              <CommandItem
                value="server-default"
                onSelect={() => handleSelect(SERVER_DEFAULT)}
              >
                <div className="flex items-center gap-2 flex-1">
                  <Brain className="h-4 w-4 text-muted-foreground" />
                  <div className="flex flex-col">
                    <span className="text-sm font-medium">Server Default</span>
                    <span className="text-xs text-muted-foreground">Use backend configuration</span>
                  </div>
                </div>
                {!value && <Check className="h-4 w-4 ml-auto" />}
              </CommandItem>
            </CommandGroup>

            {/* All LLM models grouped by provider */}
            {Object.entries(modelsByProvider).map(([providerId, { displayName, models }]) => {
              const isProviderAvailable = availableProviderIds.has(providerId);
              
              return (
                <CommandGroup 
                  key={providerId} 
                  heading={
                    <div className="flex items-center gap-1">
                      {getProviderIcon(providerId)}
                      <span>{displayName}</span>
                      {!isProviderAvailable && (
                        <span className="text-xs text-orange-500 ml-1">(Not configured)</span>
                      )}
                    </div>
                  }
                >
                  {models.map((model) => {
                    const fullId = formatFullId(providerId, model.name);
                    const isSelected = value === fullId;
                    
                    return (
                      <CommandItem
                        key={fullId}
                        value={`${model.display_name} ${model.name} ${displayName}`}
                        onSelect={() => handleSelect(fullId)}
                        disabled={!isProviderAvailable}
                        className={cn(!isProviderAvailable && "opacity-50")}
                      >
                        <div className="flex flex-col flex-1 min-w-0">
                          <div className="flex items-center gap-1.5">
                            <span className="text-sm font-medium truncate">
                              {model.display_name}
                            </span>
                            {model.capabilities?.supports_vision && (
                              <span title="Vision support">
                                <Eye className="h-3 w-3 text-blue-500 flex-shrink-0" />
                              </span>
                            )}
                            {model.capabilities?.supports_streaming && (
                              <span title="Streaming">
                                <Zap className="h-3 w-3 text-yellow-500 flex-shrink-0" />
                              </span>
                            )}
                          </div>
                          <span className="text-xs text-muted-foreground truncate">
                            {model.name}
                            {model.capabilities?.context_length && (
                              <> · {(model.capabilities.context_length / 1000).toFixed(0)}K ctx</>
                            )}
                          </span>
                        </div>
                        {isSelected && <Check className="h-4 w-4 ml-auto" />}
                      </CommandItem>
                    );
                  })}
                </CommandGroup>
              );
            })}
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}

export { formatFullId, parseFullId };

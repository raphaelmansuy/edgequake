/**
 * @module ProviderModelSelector
 * @description Dropdown selector for choosing LLM provider and model.
 * Displays available providers grouped by type with their default models.
 * 
 * @implements SPEC-032: Ollama/LM Studio provider support - Query interface selector
 * @iteration OODA #17-18 - WebUI provider selector
 * 
 * @enforces BR0301 - Selected provider must be available/configured
 * @enforces BR0302 - Model selection persists across sessions
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
import { getProviderDisplayName, useAvailableProviders } from '@/hooks/use-providers';
import { cn } from '@/lib/utils';
import { Brain, Cloud, Cpu, FlaskConical, Loader2 } from 'lucide-react';

interface ProviderModelSelectorProps {
  /** Currently selected provider ID (e.g., "openai") */
  value?: string;
  /** Callback when provider selection changes */
  onChange?: (providerId: string) => void;
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
 * Provider & Model selector component for query interface.
 * Allows users to select which LLM provider to use for queries.
 */
export function ProviderModelSelector({
  value,
  onChange,
  disabled,
  className,
}: ProviderModelSelectorProps) {
  const { data: providers, isLoading, error } = useAvailableProviders();

  if (isLoading) {
    return (
      <div className={cn('flex items-center gap-2 px-3 py-1.5 bg-muted rounded-lg', className)}>
        <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
        <span className="text-sm text-muted-foreground">Loading...</span>
      </div>
    );
  }

  if (error || !providers) {
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
            <p>Could not load providers. Using server default.</p>
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>
    );
  }

  // Get available LLM providers
  const availableProviders = providers.llm_providers.filter((p) => p.available);
  const currentProvider = value || providers.active_llm_provider;

  return (
    <Select
      value={currentProvider}
      onValueChange={onChange}
      disabled={disabled || availableProviders.length === 0}
    >
      <SelectTrigger className={cn('w-[160px] h-9', className)}>
        <SelectValue placeholder="Select provider">
          {currentProvider && (
            <div className="flex items-center gap-2">
              {getProviderIcon(currentProvider)}
              <span className="text-sm">{getProviderDisplayName(currentProvider)}</span>
            </div>
          )}
        </SelectValue>
      </SelectTrigger>
      <SelectContent>
        <SelectGroup>
          <SelectLabel className="text-xs font-semibold uppercase tracking-wide text-muted-foreground px-2">
            LLM Providers
          </SelectLabel>
          {availableProviders.map((provider) => (
            <SelectItem key={provider.id} value={provider.id}>
              <div className="flex items-center gap-2">
                {getProviderIcon(provider.id)}
                <div className="flex flex-col">
                  <span className="text-sm font-medium">{provider.name}</span>
                  <span className="text-xs text-muted-foreground">
                    {provider.default_models.chat_model}
                  </span>
                </div>
              </div>
            </SelectItem>
          ))}
        </SelectGroup>
        
        {/* Show unavailable providers as disabled */}
        {providers.llm_providers.filter((p) => !p.available).length > 0 && (
          <SelectGroup>
            <SelectLabel className="text-xs font-semibold uppercase tracking-wide text-muted-foreground px-2">
              Not Configured
            </SelectLabel>
            {providers.llm_providers
              .filter((p) => !p.available)
              .map((provider) => (
                <SelectItem key={provider.id} value={provider.id} disabled>
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

/**
 * @module ModelSelector
 * @description Enhanced dropdown selector for LLM and embedding models with
 * rich information display including capabilities, cost, and context length.
 *
 * @implements SPEC-032: Ollama/LM Studio provider support - Enhanced model selection
 * @iteration OODA #78 - Model Selector Component
 *
 * @enforces BR0301 - Selected provider must be available/configured
 * @enforces BR0302 - Model selection persists across sessions
 * @enforces BR0305 - Users can see model capabilities before selection
 */
'use client';

import { Badge } from '@/components/ui/badge';
import {
    Command,
    CommandEmpty,
    CommandGroup,
    CommandInput,
    CommandItem,
    CommandList,
    CommandSeparator,
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
import { useEmbeddingModels, useLlmModels } from '@/hooks/use-models';
import type { EmbeddingModelItem, LlmModelItem, ModelCapabilities, ModelCost } from '@/lib/api/models';
import { formatContextLength, formatCost, isModelFree } from '@/lib/api/models';
import { cn } from '@/lib/utils';
import {
    Brain,
    Check,
    ChevronDown,
    Cloud,
    Cpu,
    DollarSign,
    Eye,
    FileText,
    FlaskConical,
    Globe,
    Loader2,
    Ruler,
    Sparkles,
    Zap,
} from 'lucide-react';
import { useMemo, useState } from 'react';

import { Button } from '../ui/button';

/** A model item for display (unified for LLM and embedding). */
export interface DisplayModelItem {
  value: string;
  provider: string;
  providerDisplayName: string;
  name: string;
  displayName: string;
  description: string;
  capabilities: ModelCapabilities;
  cost?: ModelCost;
  dimension?: number;
}

/** Grouped models by provider. */
interface ModelGroup {
  provider: string;
  displayName: string;
  models: DisplayModelItem[];
}

interface ModelSelectorProps {
  /** Currently selected model (format: "provider:model") */
  value?: string;
  /** Callback when model selection changes */
  onChange?: (value: string, model: DisplayModelItem) => void;
  /** Type of models to show */
  type: 'llm' | 'embedding';
  /** Whether the selector is disabled */
  disabled?: boolean;
  /** Placeholder text */
  placeholder?: string;
  /** Additional CSS classes */
  className?: string;
  /**
   * When true (and type === 'llm'), only shows models with supports_vision === true.
   * Use this for selecting the Vision LLM used in PDF-to-Markdown extraction.
   */
  filterVision?: boolean;
}

/**
 * Get icon component for a provider.
 */
function getProviderIcon(providerId: string, className?: string) {
  const iconClass = cn('h-4 w-4', className);
  switch (providerId.toLowerCase()) {
    case 'openai':
      return <Cloud className={cn(iconClass, 'text-green-600')} />;
    case 'ollama':
      return <Cpu className={cn(iconClass, 'text-blue-600')} />;
    case 'lmstudio':
      return <Brain className={cn(iconClass, 'text-purple-600')} />;
    case 'anthropic':
      return <Sparkles className={cn(iconClass, 'text-orange-600')} />;
    case 'gemini':
      return <Zap className={cn(iconClass, 'text-blue-500')} />;
    case 'xai':
      return <Sparkles className={cn(iconClass, 'text-slate-700 dark:text-slate-300')} />;
    case 'openrouter':
      return <Globe className={cn(iconClass, 'text-indigo-600')} />;
    case 'azure':
      return <Cloud className={cn(iconClass, 'text-sky-600')} />;
    case 'minimax':
      return <Sparkles className={cn(iconClass, 'text-teal-600')} />;
    case 'mock':
      return <FlaskConical className={cn(iconClass, 'text-gray-500')} />;
    default:
      return <Brain className={cn(iconClass, 'text-muted-foreground')} />;
  }
}

/**
 * Render capability indicators for a model.
 */
function CapabilityIndicators({ capabilities }: { capabilities: ModelCapabilities }) {
  const indicators = [];

  if (capabilities.supports_vision) {
    indicators.push(
      <TooltipProvider key="vision">
        <Tooltip>
          <TooltipTrigger asChild>
            <Eye className="h-3 w-3 text-blue-500" />
          </TooltipTrigger>
          <TooltipContent>Vision support</TooltipContent>
        </Tooltip>
      </TooltipProvider>
    );
  }

  if (capabilities.supports_streaming) {
    indicators.push(
      <TooltipProvider key="streaming">
        <Tooltip>
          <TooltipTrigger asChild>
            <Zap className="h-3 w-3 text-yellow-500" />
          </TooltipTrigger>
          <TooltipContent>Streaming support</TooltipContent>
        </Tooltip>
      </TooltipProvider>
    );
  }

  return indicators.length > 0 ? (
    <div className="flex items-center gap-1 ml-1">{indicators}</div>
  ) : null;
}

/**
 * Model item in the dropdown.
 */
function ModelItem({
  model,
  selected,
  type,
}: {
  model: DisplayModelItem;
  selected: boolean;
  type: 'llm' | 'embedding';
}) {
  const isFree = isModelFree(model.cost);

  return (
    <div className="flex items-center justify-between w-full">
      <div className="flex items-center gap-2 min-w-0">
        {getProviderIcon(model.provider)}
        <div className="flex flex-col min-w-0">
          <div className="flex items-center gap-1">
            <span className="text-sm font-medium truncate">
              {model.displayName || model.name}
            </span>
            <CapabilityIndicators capabilities={model.capabilities} />
          </div>
          <span className="text-xs text-muted-foreground truncate">
            {model.providerDisplayName}
          </span>
        </div>
      </div>
      <div className="flex items-center gap-2 ml-2 shrink-0">
        {/* Cost indicator */}
        {isFree ? (
          <Badge
            variant="secondary"
            className="px-1 py-0 text-xs bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-100"
          >
            Free
          </Badge>
        ) : model.cost ? (
          type === 'embedding' && model.cost.embedding_per_1k ? (
            <Badge variant="outline" className="px-1 py-0 text-xs">
              {formatCost(model.cost.embedding_per_1k)}/1K
            </Badge>
          ) : model.cost.input_per_1k || model.cost.output_per_1k ? (
            <Badge variant="outline" className="px-1 py-0 text-xs">
              <DollarSign className="h-2.5 w-2.5" />
            </Badge>
          ) : null
        ) : null}

        {/* Context/Dimension */}
        {type === 'llm' && model.capabilities.context_length > 0 && (
          <Badge variant="outline" className="px-1 py-0 text-xs gap-0.5">
            <FileText className="h-2.5 w-2.5" />
            {formatContextLength(model.capabilities.context_length)}
          </Badge>
        )}
        {type === 'embedding' && model.dimension && model.dimension > 0 && (
          <Badge variant="outline" className="px-1 py-0 text-xs gap-0.5">
            <Ruler className="h-2.5 w-2.5" />
            {model.dimension}d
          </Badge>
        )}

        {/* Selection check */}
        {selected && <Check className="h-4 w-4 text-primary shrink-0" />}
      </div>
    </div>
  );
}

/** Convert LlmModelItem to DisplayModelItem */
function llmToDisplayItem(item: LlmModelItem): DisplayModelItem {
  return {
    value: `${item.provider}:${item.name}`,
    provider: item.provider,
    providerDisplayName: item.provider_display_name,
    name: item.name,
    displayName: item.display_name,
    description: item.description,
    capabilities: item.capabilities,
    cost: item.cost,
  };
}

/** Convert EmbeddingModelItem to DisplayModelItem */
function embeddingToDisplayItem(item: EmbeddingModelItem): DisplayModelItem {
  return {
    value: `${item.provider}:${item.name}`,
    provider: item.provider,
    providerDisplayName: item.provider_display_name,
    name: item.name,
    displayName: item.display_name,
    description: item.description,
    capabilities: item.capabilities,
    cost: item.cost,
    dimension: item.dimension,
  };
}

/**
 * Enhanced model selector with rich information display.
 */
export function ModelSelector({
  value,
  onChange,
  type,
  disabled,
  placeholder,
  className,
  filterVision = false,
}: ModelSelectorProps) {
  const [open, setOpen] = useState(false);

  const { data: llmData, isLoading: llmLoading } = useLlmModels();
  const { data: embeddingData, isLoading: embeddingLoading } = useEmbeddingModels();

  const isLoading = type === 'llm' ? llmLoading : embeddingLoading;

  // Build grouped options
  const groups = useMemo<ModelGroup[]>(() => {
    if (type === 'llm' && llmData) {
      const groupMap = new Map<string, ModelGroup>();
      const llmModels = filterVision
        ? llmData.models.filter((m) => m.capabilities.supports_vision)
        : llmData.models;
      for (const model of llmModels) {
        const existing = groupMap.get(model.provider);
        const displayItem = llmToDisplayItem(model);
        if (existing) {
          existing.models.push(displayItem);
        } else {
          groupMap.set(model.provider, {
            provider: model.provider,
            displayName: model.provider_display_name,
            models: [displayItem],
          });
        }
      }
      return Array.from(groupMap.values());
    }
    if (type === 'embedding' && embeddingData) {
      const groupMap = new Map<string, ModelGroup>();
      for (const model of embeddingData.models) {
        const existing = groupMap.get(model.provider);
        const displayItem = embeddingToDisplayItem(model);
        if (existing) {
          existing.models.push(displayItem);
        } else {
          groupMap.set(model.provider, {
            provider: model.provider,
            displayName: model.provider_display_name,
            models: [displayItem],
          });
        }
      }
      return Array.from(groupMap.values());
    }
    return [];
  }, [type, llmData, embeddingData, filterVision]);

  // Find selected model
  const selectedModel = useMemo(() => {
    if (!value) return null;
    for (const group of groups) {
      const found = group.models.find((m) => m.value === value);
      if (found) return found;
    }
    return null;
  }, [value, groups]);

  const handleSelect = (selectedValue: string) => {
    for (const group of groups) {
      const found = group.models.find((m) => m.value === selectedValue);
      if (found) {
        onChange?.(selectedValue, found);
        setOpen(false);
        return;
      }
    }
  };

  if (isLoading) {
    return (
      <Button
        variant="outline"
        className={cn('w-full justify-start gap-2', className)}
        disabled
      >
        <Loader2 className="h-4 w-4 animate-spin" />
        <span className="text-muted-foreground">Loading models...</span>
      </Button>
    );
  }

  if (groups.length === 0) {
    return (
      <Button
        variant="outline"
        className={cn('w-full justify-start gap-2', className)}
        disabled
      >
        <Brain className="h-4 w-4 text-muted-foreground" />
        <span className="text-muted-foreground">No models available</span>
      </Button>
    );
  }

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          role="combobox"
          aria-expanded={open}
          disabled={disabled}
          className={cn('w-full justify-between', className)}
        >
          {selectedModel ? (
            <div className="flex items-center gap-2 min-w-0">
              {getProviderIcon(selectedModel.provider)}
              <span className="truncate">
                {selectedModel.displayName || selectedModel.name}
              </span>
              {type === 'embedding' && selectedModel.dimension && (
                <Badge variant="secondary" className="px-1 py-0 text-xs">
                  {selectedModel.dimension}d
                </Badge>
              )}
            </div>
          ) : (
            <span className="text-muted-foreground">
              {placeholder || `Select ${type === 'llm' ? 'LLM' : 'embedding'} model...`}
            </span>
          )}
          <ChevronDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-[400px] p-0" align="start">
        <Command>
          <CommandInput placeholder={`Search ${type} models...`} />
          <CommandList>
            <CommandEmpty>No models found.</CommandEmpty>
            {groups.map((group, idx) => (
              <div key={group.provider}>
                {idx > 0 && <CommandSeparator />}
                <CommandGroup heading={group.displayName}>
                  {group.models.map((model) => (
                    <CommandItem
                      key={model.value}
                      value={model.value}
                      onSelect={handleSelect}
                      className="cursor-pointer"
                    >
                      <ModelItem
                        model={model}
                        selected={value === model.value}
                        type={type}
                      />
                    </CommandItem>
                  ))}
                </CommandGroup>
              </div>
            ))}
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}

/**
 * Simplified LLM model selector.
 */
export function LlmModelSelector({
  value,
  onChange,
  disabled,
  className,
}: Omit<ModelSelectorProps, 'type' | 'placeholder'>) {
  return (
    <ModelSelector
      value={value}
      onChange={onChange}
      type="llm"
      disabled={disabled}
      placeholder="Select LLM..."
      className={className}
    />
  );
}

/**
 * Simplified embedding model selector.
 */
export function EmbeddingModelSelector2({
  value,
  onChange,
  disabled,
  className,
}: Omit<ModelSelectorProps, 'type' | 'placeholder'>) {
  return (
    <ModelSelector
      value={value}
      onChange={onChange}
      type="embedding"
      disabled={disabled}
      placeholder="Select embedding model..."
      className={className}
    />
  );
}

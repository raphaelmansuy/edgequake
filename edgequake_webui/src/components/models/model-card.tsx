/**
 * @module ModelCard
 * @description Card component displaying model information including
 * capabilities, cost, context length, and other metadata.
 *
 * @implements SPEC-032: Ollama/LM Studio provider support - Model information display
 * @iteration OODA #77 - Model Cards Component
 *
 * @enforces BR0305 - Users can see model capabilities before selection
 * @enforces BR0306 - Cost information is clearly displayed
 */
'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
} from '@/components/ui/card';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import type { ModelCost, ModelResponse } from '@/lib/api/models';
import { formatContextLength, formatCost } from '@/lib/api/models';
import { cn } from '@/lib/utils';
import {
    Brain,
    Check,
    Cloud,
    Cpu,
    DollarSign,
    FileText,
    FlaskConical,
    Ruler,
    Sparkles,
} from 'lucide-react';

import { ModelCapabilitiesDisplay } from './model-capability-badge';

interface ModelCardProps {
  model: ModelResponse;
  selected?: boolean;
  onSelect?: (model: ModelResponse) => void;
  compact?: boolean;
  className?: string;
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
    case 'azure':
      return <Cloud className={cn(iconClass, 'text-blue-500')} />;
    case 'mock':
      return <FlaskConical className={cn(iconClass, 'text-gray-500')} />;
    default:
      return <Brain className={cn(iconClass, 'text-muted-foreground')} />;
  }
}

/**
 * Format cost display for a model.
 */
function CostDisplay({ cost, modelType }: { cost?: ModelCost; modelType: string }) {
  if (!cost) {
    return (
      <Badge variant="secondary" className="gap-1">
        <DollarSign className="h-3 w-3" />
        <span>Free</span>
      </Badge>
    );
  }

  const isFree =
    (!cost.input_per_1k || cost.input_per_1k === 0) &&
    (!cost.output_per_1k || cost.output_per_1k === 0) &&
    (!cost.embedding_per_1k || cost.embedding_per_1k === 0);

  if (isFree) {
    return (
      <Badge variant="secondary" className="gap-1 bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-100">
        <DollarSign className="h-3 w-3" />
        <span>Free</span>
      </Badge>
    );
  }

  if (modelType === 'embedding' && cost.embedding_per_1k) {
    return (
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <Badge variant="outline" className="gap-1 cursor-help">
              <DollarSign className="h-3 w-3" />
              <span>{formatCost(cost.embedding_per_1k)}/1K</span>
            </Badge>
          </TooltipTrigger>
          <TooltipContent>
            <p className="text-sm">Cost per 1,000 tokens for embedding</p>
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>
    );
  }

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <Badge variant="outline" className="gap-1 cursor-help">
            <DollarSign className="h-3 w-3" />
            <span>
              {formatCost(cost.input_per_1k || 0)}/{formatCost(cost.output_per_1k || 0)}
            </span>
          </Badge>
        </TooltipTrigger>
        <TooltipContent>
          <div className="space-y-1">
            <p className="text-sm font-medium">Cost per 1K tokens</p>
            <p className="text-xs">Input: {formatCost(cost.input_per_1k || 0)}</p>
            <p className="text-xs">Output: {formatCost(cost.output_per_1k || 0)}</p>
            {cost.image_per_unit && (
              <p className="text-xs">Image: {formatCost(cost.image_per_unit)}/image</p>
            )}
          </div>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

/**
 * Context length display with formatting.
 */
function ContextDisplay({ contextLength }: { contextLength?: number }) {
  if (!contextLength) {
    return null;
  }

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <Badge variant="outline" className="gap-1 cursor-help">
            <FileText className="h-3 w-3" />
            <span>{formatContextLength(contextLength)}</span>
          </Badge>
        </TooltipTrigger>
        <TooltipContent>
          <p className="text-sm">
            Context window: {contextLength.toLocaleString()} tokens
          </p>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

/**
 * Embedding dimension display.
 */
function DimensionDisplay({ dimension }: { dimension?: number }) {
  if (!dimension) {
    return null;
  }

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <Badge variant="outline" className="gap-1 cursor-help">
            <Ruler className="h-3 w-3" />
            <span>{dimension}d</span>
          </Badge>
        </TooltipTrigger>
        <TooltipContent>
          <p className="text-sm">Embedding dimension: {dimension}</p>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

/**
 * Model card component displaying comprehensive model information.
 */
export function ModelCard({
  model,
  selected,
  onSelect,
  compact = false,
  className,
}: ModelCardProps) {
  const modelType = model.model_type;

  if (compact) {
    return (
      <div
        className={cn(
          'flex items-center justify-between p-3 rounded-lg border transition-colors',
          selected
            ? 'border-primary bg-primary/5'
            : 'border-border hover:border-primary/50 hover:bg-muted/50',
          onSelect && 'cursor-pointer',
          className
        )}
        onClick={() => onSelect?.(model)}
        role={onSelect ? 'button' : undefined}
        tabIndex={onSelect ? 0 : undefined}
      >
        <div className="flex items-center gap-3">
          {getProviderIcon(model.provider)}
          <div>
            <div className="flex items-center gap-2">
              <span className="font-medium text-sm">{model.display_name || model.name}</span>
              {selected && <Check className="h-4 w-4 text-primary" />}
            </div>
            <span className="text-xs text-muted-foreground">{model.provider}</span>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <CostDisplay cost={model.cost} modelType={modelType} />
          {modelType === 'embedding' ? (
            <DimensionDisplay dimension={model.capabilities.embedding_dimension} />
          ) : (
            <ContextDisplay contextLength={model.capabilities.context_length} />
          )}
        </div>
      </div>
    );
  }

  return (
    <Card
      className={cn(
        'transition-all',
        selected && 'ring-2 ring-primary',
        onSelect && 'cursor-pointer hover:shadow-md',
        className
      )}
      onClick={() => onSelect?.(model)}
    >
      <CardHeader className="pb-3">
        <div className="flex items-start justify-between">
          <div className="flex items-center gap-2">
            {getProviderIcon(model.provider, 'h-5 w-5')}
            <div>
              <CardTitle className="text-base">
                {model.display_name || model.name}
                {selected && <Check className="ml-2 h-4 w-4 inline text-primary" />}
              </CardTitle>
              <CardDescription className="text-xs">
                {model.provider} • {model.name}
              </CardDescription>
            </div>
          </div>
          <Badge
            variant={modelType === 'llm' ? 'default' : 'secondary'}
            className="capitalize"
          >
            {modelType}
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="space-y-3">
        {/* Description */}
        {model.description && (
          <p className="text-sm text-muted-foreground line-clamp-2">
            {model.description}
          </p>
        )}

        {/* Metrics row */}
        <div className="flex flex-wrap gap-2">
          <CostDisplay cost={model.cost} modelType={modelType} />
          <ContextDisplay contextLength={model.capabilities.context_length} />
          {model.capabilities.max_output_tokens && (
            <Badge variant="outline" className="gap-1">
              <FileText className="h-3 w-3" />
              <span>Max {formatContextLength(model.capabilities.max_output_tokens)}</span>
            </Badge>
          )}
          <DimensionDisplay dimension={model.capabilities.embedding_dimension} />
        </div>

        {/* Capabilities */}
        {modelType !== 'embedding' && (
          <ModelCapabilitiesDisplay capabilities={model.capabilities} />
        )}

        {/* Tags */}
        {model.tags && model.tags.length > 0 && (
          <div className="flex flex-wrap gap-1">
            {model.tags.slice(0, 5).map((tag) => (
              <Badge key={tag} variant="outline" className="text-xs px-1.5 py-0">
                {tag}
              </Badge>
            ))}
            {model.tags.length > 5 && (
              <Badge variant="outline" className="text-xs px-1.5 py-0">
                +{model.tags.length - 5}
              </Badge>
            )}
          </div>
        )}

        {/* Select button */}
        {onSelect && (
          <Button
            variant={selected ? 'default' : 'outline'}
            size="sm"
            className="w-full"
            onClick={(e) => {
              e.stopPropagation();
              onSelect(model);
            }}
          >
            {selected ? 'Selected' : 'Select'}
          </Button>
        )}
      </CardContent>
    </Card>
  );
}

/**
 * Grid of model cards for selection.
 */
interface ModelCardGridProps {
  models: ModelResponse[];
  selectedModel?: string;
  onSelect?: (model: ModelResponse) => void;
  compact?: boolean;
  className?: string;
}

export function ModelCardGrid({
  models,
  selectedModel,
  onSelect,
  compact = false,
  className,
}: ModelCardGridProps) {
  if (compact) {
    return (
      <div className={cn('space-y-2', className)}>
        {models.map((model) => (
          <ModelCard
            key={`${model.provider}:${model.name}`}
            model={model}
            selected={selectedModel === model.name}
            onSelect={onSelect}
            compact
          />
        ))}
      </div>
    );
  }

  return (
    <div
      className={cn(
        'grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4',
        className
      )}
    >
      {models.map((model) => (
        <ModelCard
          key={`${model.provider}:${model.name}`}
          model={model}
          selected={selectedModel === model.name}
          onSelect={onSelect}
        />
      ))}
    </div>
  );
}

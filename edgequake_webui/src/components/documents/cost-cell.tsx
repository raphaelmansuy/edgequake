/**
 * Cost Cell Component
 * 
 * Enhanced table cell for displaying document processing costs with token breakdown.
 * Provides rich tooltip with input/output token counts and model information.
 */

'use client';

import { Badge } from '@/components/ui/badge';
import {
    Tooltip,
    TooltipContent,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { cn } from '@/lib/utils';
import type { Document } from '@/types';
import { Coins, Cpu, DollarSign, FileText, Zap } from 'lucide-react';

interface CostCellProps {
  /** Document with cost data */
  document: Document;
  /** Size variant */
  size?: 'sm' | 'md' | 'lg';
  /** Custom class name */
  className?: string;
}

/**
 * Formats cost as USD string with appropriate precision.
 */
function formatCost(cost: number | undefined): string {
  if (cost === undefined || cost === null) return '-';
  if (cost === 0) return '$0.00';
  if (cost < 0.0001) return '<$0.0001';
  if (cost < 0.001) return `$${cost.toFixed(5)}`;
  if (cost < 0.01) return `$${cost.toFixed(4)}`;
  if (cost < 1) return `$${cost.toFixed(3)}`;
  return `$${cost.toFixed(2)}`;
}

/**
 * Formats token count with K/M suffix.
 */
function formatTokens(tokens: number | undefined): string {
  if (tokens === undefined || tokens === null) return '-';
  if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(1)}M`;
  if (tokens >= 10_000) return `${(tokens / 1000).toFixed(0)}K`;
  if (tokens >= 1000) return `${(tokens / 1000).toFixed(1)}K`;
  return tokens.toLocaleString();
}

/**
 * Get color class based on cost amount.
 */
function getCostColor(cost: number | undefined): string {
  if (cost === undefined || cost === null || cost === 0) return 'text-muted-foreground';
  if (cost < 0.001) return 'text-green-600 dark:text-green-400';
  if (cost < 0.01) return 'text-blue-600 dark:text-blue-400';
  if (cost < 0.1) return 'text-yellow-600 dark:text-yellow-400';
  return 'text-orange-600 dark:text-orange-400';
}

const sizeStyles = {
  sm: 'text-xs px-1.5 py-0.5',
  md: 'text-sm px-2 py-1',
  lg: 'text-base px-3 py-1.5',
};

const iconSizes = {
  sm: 'h-3 w-3',
  md: 'h-3.5 w-3.5',
  lg: 'h-4 w-4',
};

/**
 * Displays document processing cost with detailed breakdown tooltip.
 * 
 * Features:
 * - Color-coded cost display
 * - Token breakdown (input/output/total)
 * - Model information
 * - Responsive sizing
 */
export function CostCell({
  document,
  size = 'sm',
  className,
}: CostCellProps) {
  const { cost_usd, input_tokens, output_tokens, total_tokens, llm_model, embedding_model } = document;
  const hasCostData = cost_usd !== undefined && cost_usd !== null;
  const hasTokenData = total_tokens !== undefined && total_tokens !== null && total_tokens > 0;

  // If no cost data, show a placeholder
  if (!hasCostData && !hasTokenData) {
    return (
      <span className={cn('text-muted-foreground', sizeStyles[size], className)}>
        -
      </span>
    );
  }

  const badge = (
    <Badge
      variant="outline"
      className={cn(
        'inline-flex items-center gap-1 font-mono cursor-help',
        getCostColor(cost_usd),
        sizeStyles[size],
        className
      )}
    >
      <DollarSign className={cn(iconSizes[size], 'opacity-70')} />
      <span>{formatCost(cost_usd).replace('$', '')}</span>
    </Badge>
  );

  return (
    <Tooltip>
      <TooltipTrigger asChild>{badge}</TooltipTrigger>
      <TooltipContent className="w-72 p-0" side="left">
        <CostBreakdownTooltip
          cost_usd={cost_usd}
          input_tokens={input_tokens}
          output_tokens={output_tokens}
          total_tokens={total_tokens}
          llm_model={llm_model}
          embedding_model={embedding_model}
          document_title={document.title || document.file_name}
        />
      </TooltipContent>
    </Tooltip>
  );
}

/**
 * Detailed cost breakdown tooltip content.
 */
interface CostBreakdownTooltipProps {
  cost_usd?: number;
  input_tokens?: number;
  output_tokens?: number;
  total_tokens?: number;
  llm_model?: string;
  embedding_model?: string;
  document_title?: string | null;
}

function CostBreakdownTooltip({
  cost_usd,
  input_tokens,
  output_tokens,
  total_tokens,
  llm_model,
  embedding_model,
  document_title,
}: CostBreakdownTooltipProps) {
  return (
    <div className="divide-y">
      {/* Header */}
      <div className="px-3 py-2 bg-muted/50">
        <div className="flex items-center gap-2">
          <Coins className="h-4 w-4 text-primary" />
          <span className="font-semibold text-sm">Cost Breakdown</span>
        </div>
        {document_title && (
          <p className="text-xs text-muted-foreground mt-1 truncate">
            {document_title}
          </p>
        )}
      </div>

      {/* Cost Summary */}
      <div className="p-3 space-y-2">
        <div className="flex items-center justify-between">
          <span className="text-sm text-muted-foreground flex items-center gap-1.5">
            <DollarSign className="h-3.5 w-3.5" />
            Total Cost
          </span>
          <span className={cn('font-mono font-medium', getCostColor(cost_usd))}>
            {formatCost(cost_usd)}
          </span>
        </div>
      </div>

      {/* Token Breakdown */}
      <div className="p-3 space-y-2">
        <div className="text-xs font-medium text-muted-foreground mb-2 flex items-center gap-1.5">
          <Zap className="h-3 w-3" />
          Token Usage
        </div>
        
        <div className="grid grid-cols-2 gap-2 text-sm">
          <div className="flex items-center justify-between">
            <span className="text-muted-foreground text-xs">Input</span>
            <span className="font-mono text-xs">{formatTokens(input_tokens)}</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-muted-foreground text-xs">Output</span>
            <span className="font-mono text-xs">{formatTokens(output_tokens)}</span>
          </div>
        </div>
        
        <div className="flex items-center justify-between border-t pt-2 mt-2">
          <span className="text-muted-foreground text-xs">Total</span>
          <span className="font-mono text-xs font-medium">{formatTokens(total_tokens)}</span>
        </div>
      </div>

      {/* Model Information */}
      {(llm_model || embedding_model) && (
        <div className="p-3 space-y-2">
          <div className="text-xs font-medium text-muted-foreground mb-2 flex items-center gap-1.5">
            <Cpu className="h-3 w-3" />
            Models
          </div>
          
          {llm_model && (
            <div className="flex items-center justify-between">
              <span className="text-muted-foreground text-xs flex items-center gap-1">
                <FileText className="h-3 w-3" />
                LLM
              </span>
              <span className="font-mono text-xs truncate max-w-[140px]" title={llm_model}>
                {llm_model.split('/').pop() || llm_model}
              </span>
            </div>
          )}
          
          {embedding_model && (
            <div className="flex items-center justify-between">
              <span className="text-muted-foreground text-xs flex items-center gap-1">
                <Zap className="h-3 w-3" />
                Embedding
              </span>
              <span className="font-mono text-xs truncate max-w-[140px]" title={embedding_model}>
                {embedding_model.split('/').pop() || embedding_model}
              </span>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default CostCell;

/**
 * Cost Badge Component
 * 
 * Inline cost display with optional breakdown tooltip.
 * Based on WebUI Specification Document WEBUI-004 (13-webui-components.md)
 */

'use client';

import { Badge } from '@/components/ui/badge';
import {
    Tooltip,
    TooltipContent,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { cn } from '@/lib/utils';
import type { CostBreakdown, StageCostBreakdown } from '@/types/cost';
import { DollarSign, TrendingDown, TrendingUp } from 'lucide-react';

interface CostBadgeProps {
  /** Cost in USD */
  cost: number;
  /** Estimated final cost in USD */
  estimated?: number;
  /** Show breakdown tooltip on hover */
  showBreakdown?: boolean;
  /** Cost breakdown data */
  breakdown?: CostBreakdown;
  /** Size variant */
  size?: 'sm' | 'md' | 'lg';
  /** Show trend indicator against estimate */
  showTrend?: boolean;
  /** Custom class name */
  className?: string;
}

/**
 * Formats cost as USD string.
 */
function formatCost(cost: number): string {
  if (cost === 0) return '$0.00';
  if (cost < 0.0001) return '<$0.0001';
  if (cost < 0.01) return `$${cost.toFixed(4)}`;
  if (cost < 1) return `$${cost.toFixed(3)}`;
  return `$${cost.toFixed(2)}`;
}

/**
 * Formats token count with K/M suffix.
 */
function formatTokens(tokens: number): string {
  if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(1)}M`;
  if (tokens >= 1000) return `${(tokens / 1000).toFixed(1)}K`;
  return tokens.toString();
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
 * Displays cost in USD with optional breakdown tooltip.
 * 
 * Features:
 * - Multiple size variants
 * - Estimated cost comparison
 * - Detailed breakdown on hover
 * - Trend indicator
 */
export function CostBadge({
  cost,
  estimated,
  showBreakdown = false,
  breakdown,
  size = 'md',
  showTrend = false,
  className,
}: CostBadgeProps) {
  // Calculate trend if we have both cost and estimate
  const trend = estimated && cost > 0
    ? ((cost - estimated) / estimated) * 100
    : null;

  const TrendIcon = trend && trend > 0 ? TrendingUp : TrendingDown;
  const trendColor = trend && trend > 10 ? 'text-red-500' : trend && trend < -10 ? 'text-green-500' : 'text-muted-foreground';

  const badge = (
    <Badge
      variant="secondary"
      className={cn(
        'inline-flex items-center gap-1 font-mono',
        sizeStyles[size],
        className
      )}
    >
      <DollarSign className={cn(iconSizes[size], 'text-muted-foreground')} />
      <span>{formatCost(cost).replace('$', '')}</span>
      
      {/* Estimated cost */}
      {estimated !== undefined && size === 'lg' && (
        <span className="text-muted-foreground">
          / {formatCost(estimated)}
        </span>
      )}
      
      {/* Trend indicator */}
      {showTrend && trend !== null && Math.abs(trend) > 5 && (
        <TrendIcon className={cn('h-3 w-3', trendColor)} />
      )}
    </Badge>
  );

  // Without breakdown tooltip
  if (!showBreakdown || !breakdown) {
    return (
      <Tooltip>
        <TooltipTrigger asChild>{badge}</TooltipTrigger>
        <TooltipContent>
          <p>Cost: {formatCost(cost)}</p>
          {estimated && <p>Estimated: {formatCost(estimated)}</p>}
        </TooltipContent>
      </Tooltip>
    );
  }

  // With detailed breakdown tooltip
  return (
    <Tooltip>
      <TooltipTrigger asChild>{badge}</TooltipTrigger>
      <TooltipContent className="w-64 p-0">
        <CostBreakdownTooltip breakdown={breakdown} />
      </TooltipContent>
    </Tooltip>
  );
}

/**
 * Detailed cost breakdown tooltip content.
 */
function CostBreakdownTooltip({ breakdown }: { breakdown: CostBreakdown }) {
  return (
    <div className="p-3">
      <h4 className="font-semibold text-sm mb-2">Cost Breakdown</h4>
      <div className="space-y-1.5 text-xs">
        {/* By stage */}
        {breakdown.by_stage && breakdown.by_stage.length > 0 && (
          <>
            <div className="border-b pb-1 mb-1 text-muted-foreground">
              By Stage
            </div>
            {breakdown.by_stage.map((stage: StageCostBreakdown) => (
              <div key={stage.stage} className="flex justify-between">
                <span className="capitalize">{stage.stage}</span>
                <span className="font-mono">{formatCost(stage.cost)}</span>
              </div>
            ))}
          </>
        )}

        {/* Token usage */}
        {breakdown.tokens && (
          <>
            <div className="border-b pb-1 mb-1 mt-2 text-muted-foreground">
              Token Usage
            </div>
            <div className="flex justify-between">
              <span>Input</span>
              <span className="font-mono">{formatTokens(breakdown.tokens.input)}</span>
            </div>
            <div className="flex justify-between">
              <span>Output</span>
              <span className="font-mono">{formatTokens(breakdown.tokens.output)}</span>
            </div>
            <div className="flex justify-between">
              <span>Total</span>
              <span className="font-mono">{formatTokens(breakdown.tokens.total)}</span>
            </div>
          </>
        )}

        {/* Total */}
        <div className="border-t pt-1.5 mt-2 flex justify-between font-semibold">
          <span>Total</span>
          <span className="font-mono">{formatCost(breakdown.total_cost)}</span>
        </div>
      </div>
    </div>
  );
}

/**
 * Minimal cost display for compact layouts.
 */
export function CostInline({
  cost,
  className,
}: {
  cost: number;
  className?: string;
}) {
  return (
    <span className={cn('text-xs font-mono text-muted-foreground', className)}>
      {formatCost(cost)}
    </span>
  );
}

export default CostBadge;

/**
 * Cost Summary Card Component
 * 
 * Overview card displaying cost summary statistics.
 * Based on WebUI Specification Document WEBUI-007 (16-webui-cost-monitoring.md)
 *
 * @implements FEAT1044 - Cost summary statistics display
 * @implements FEAT1045 - Token usage aggregation
 *
 * @see UC1205 - User views total cost and token usage
 * @see UC1206 - User reviews cost efficiency metrics
 *
 * @enforces BR1044 - Smart number formatting (K/M suffixes)
 * @enforces BR1045 - Date range display for cost periods
 */

'use client';

import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
} from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { cn } from '@/lib/utils';
import type { CostSummary } from '@/types/cost';
import { DollarSign, FileText, Hash, TrendingUp } from 'lucide-react';

interface CostSummaryCardProps {
  /** Cost summary data */
  summary: CostSummary | null;
  /** Loading state */
  isLoading?: boolean;
  /** Custom class name */
  className?: string;
}

/**
 * Formats cost as USD string.
 */
function formatCost(cost: number): string {
  if (cost === 0) return '$0.00';
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

/**
 * Formats date range for display.
 */
function formatDateRange(startDate?: string, endDate?: string): string {
  if (!startDate || !endDate) return 'All time';
  
  const start = new Date(startDate);
  const end = new Date(endDate);
  
  const format = (date: Date) => date.toLocaleDateString(undefined, {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
  });
  
  return `${format(start)} - ${format(end)}`;
}

/**
 * Displays a summary card with key cost metrics.
 */
export function CostSummaryCard({
  summary,
  isLoading = false,
  className,
}: CostSummaryCardProps) {
  if (isLoading) {
    return (
      <Card className={className}>
        <CardHeader className="pb-2">
          <Skeleton className="h-6 w-32" />
          <Skeleton className="h-4 w-48" />
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-2 gap-4">
            {Array.from({ length: 4 }).map((_, i) => (
              <div key={i} className="space-y-2">
                <Skeleton className="h-8 w-24" />
                <Skeleton className="h-3 w-16" />
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    );
  }

  if (!summary) {
    return (
      <Card className={className}>
        <CardContent className="py-8 text-center text-muted-foreground">
          No cost data available
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className={className}>
      <CardHeader className="pb-2">
        <CardTitle className="text-lg flex items-center gap-2">
          <DollarSign className="h-5 w-5" />
          Cost Summary
        </CardTitle>
        <CardDescription>
          {formatDateRange(summary.period_start, summary.period_end)}
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="grid grid-cols-2 gap-4">
          {/* Total Cost */}
          <StatItem
            label="Total Cost"
            value={formatCost(summary.total_cost)}
            icon={DollarSign}
            size="large"
          />

          {/* Documents Processed */}
          <StatItem
            label="Documents"
            value={summary.document_count.toString()}
            icon={FileText}
            size="large"
          />

          {/* Average Cost */}
          <StatItem
            label="Avg per Document"
            value={formatCost(summary.average_cost_per_document)}
            icon={TrendingUp}
          />

          {/* Total Tokens */}
          <StatItem
            label="Tokens Used"
            value={formatTokens(summary.total_tokens)}
            icon={Hash}
          />
        </div>
      </CardContent>
    </Card>
  );
}

/**
 * Individual stat item within the card.
 */
function StatItem({
  label,
  value,
  icon: Icon,
  size = 'default',
}: {
  label: string;
  value: string;
  icon: React.ComponentType<{ className?: string }>;
  size?: 'default' | 'large';
}) {
  return (
    <div className="space-y-1">
      <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
        <Icon className="h-3 w-3" />
        {label}
      </div>
      <div
        className={cn(
          'font-bold font-mono',
          size === 'large' ? 'text-2xl' : 'text-lg'
        )}
      >
        {value}
      </div>
    </div>
  );
}

export default CostSummaryCard;

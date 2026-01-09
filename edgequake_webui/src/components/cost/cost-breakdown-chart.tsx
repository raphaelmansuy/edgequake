/**
 * Cost Breakdown Chart Component
 * 
 * Visual cost breakdown with pie or bar chart.
 * Based on WebUI Specification Document WEBUI-007 (16-webui-cost-monitoring.md)
 *
 * @implements FEAT1042 - Cost breakdown visualization
 * @implements FEAT1043 - Stage-wise cost categorization
 *
 * @see UC1203 - User analyzes cost by processing stage
 * @see UC1204 - User compares extraction vs embedding costs
 *
 * @enforces BR1042 - Color-coded stage categories
 * @enforces BR1043 - Dynamic chart type selection (pie/bar)
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
import type { CostBreakdown } from '@/types/cost';
import { useMemo } from 'react';

interface CostBreakdownChartProps {
  /** Cost breakdown data */
  breakdown: CostBreakdown | null;
  /** Chart type */
  type?: 'pie' | 'bar';
  /** Show legend */
  showLegend?: boolean;
  /** Show values */
  showValues?: boolean;
  /** Chart height in pixels */
  height?: number;
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

// Color palette for stages
const STAGE_COLORS: Record<string, string> = {
  extraction: '#3b82f6', // blue
  extracting: '#3b82f6',
  gleaning: '#22c55e', // green
  summarizing: '#f59e0b', // amber
  summarization: '#f59e0b',
  embedding: '#8b5cf6', // purple
  indexing: '#6366f1', // indigo
  preprocessing: '#64748b', // slate
  chunking: '#14b8a6', // teal
  merging: '#ec4899', // pink
};

const DEFAULT_COLOR = '#94a3b8';

/**
 * Visualizes cost breakdown by operation type.
 * 
 * Note: This is a simplified implementation using CSS.
 * For production, consider using Recharts for more advanced charts.
 */
export function CostBreakdownChart({
  breakdown,
  type = 'bar',
  showLegend = true,
  showValues = true,
  height = 200,
  isLoading = false,
  className,
}: CostBreakdownChartProps) {
  // Transform data for rendering
  const chartData = useMemo(() => {
    if (!breakdown?.by_stage) return [];
    
    const total = breakdown.total_cost;
    if (total === 0) return [];

    return breakdown.by_stage.map(stage => ({
      name: stage.stage,
      value: stage.cost,
      percentage: (stage.cost / total) * 100,
      color: STAGE_COLORS[stage.stage.toLowerCase()] || DEFAULT_COLOR,
    }));
  }, [breakdown]);

  if (isLoading) {
    return (
      <Card className={className}>
        <CardHeader className="pb-2">
          <Skeleton className="h-5 w-32" />
        </CardHeader>
        <CardContent>
          <Skeleton className="w-full" style={{ height }} />
        </CardContent>
      </Card>
    );
  }

  if (!breakdown || chartData.length === 0) {
    return (
      <Card className={className}>
        <CardContent className="py-8 text-center text-muted-foreground">
          No cost breakdown data available
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className={className}>
      <CardHeader className="pb-2">
        <CardTitle className="text-base">Cost Breakdown</CardTitle>
        <CardDescription>
          Total: {formatCost(breakdown.total_cost)}
        </CardDescription>
      </CardHeader>
      <CardContent>
        {type === 'bar' ? (
          <BarChart data={chartData} height={height} showValues={showValues} />
        ) : (
          <PieChart data={chartData} height={height} showValues={showValues} />
        )}

        {showLegend && (
          <div className="mt-4 flex flex-wrap gap-3">
            {chartData.map(item => (
              <div key={item.name} className="flex items-center gap-1.5">
                <div
                  className="w-3 h-3 rounded-sm"
                  style={{ backgroundColor: item.color }}
                />
                <span className="text-xs capitalize">{item.name}</span>
                <span className="text-xs text-muted-foreground">
                  ({Math.round(item.percentage)}%)
                </span>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

/**
 * Simple bar chart implementation.
 */
function BarChart({
  data,
  height,
  showValues,
}: {
  data: Array<{ name: string; value: number; percentage: number; color: string }>;
  height: number;
  showValues: boolean;
}) {
  const maxPercentage = Math.max(...data.map(d => d.percentage), 1);

  return (
    <div className="space-y-2" style={{ height }}>
      {data.map(item => (
        <div key={item.name} className="space-y-1">
          <div className="flex items-center justify-between text-sm">
            <span className="capitalize">{item.name}</span>
            {showValues && (
              <span className="font-mono text-muted-foreground">
                {formatCost(item.value)}
              </span>
            )}
          </div>
          <div className="h-6 bg-muted rounded overflow-hidden">
            <div
              className="h-full rounded transition-all duration-500"
              style={{
                width: `${(item.percentage / maxPercentage) * 100}%`,
                backgroundColor: item.color,
              }}
            />
          </div>
        </div>
      ))}
    </div>
  );
}

/**
 * Simple pie chart implementation using conic-gradient.
 */
function PieChart({
  data,
  height,
  showValues: _showValues, // eslint-disable-line @typescript-eslint/no-unused-vars
}: {
  data: Array<{ name: string; value: number; percentage: number; color: string }>;
  height: number;
  showValues: boolean;
}) {
  // Build conic gradient
  const gradient = useMemo(() => {
    const segments = data.reduce<{ angle: number; segments: string[] }>(
      (acc, item) => {
        const startAngle = acc.angle;
        const endAngle = acc.angle + (item.percentage / 100) * 360;
        acc.segments.push(`${item.color} ${startAngle}deg ${endAngle}deg`);
        acc.angle = endAngle;
        return acc;
      },
      { angle: 0, segments: [] }
    );
    return `conic-gradient(${segments.segments.join(', ')})`;
  }, [data]);

  return (
    <div className="flex items-center justify-center" style={{ height }}>
      <div
        className="rounded-full"
        style={{
          width: Math.min(height, 180),
          height: Math.min(height, 180),
          background: gradient,
        }}
      />
    </div>
  );
}

export default CostBreakdownChart;

/**
 * Cost Dashboard Page
 * 
 * Full-page cost monitoring dashboard.
 * Based on WebUI Specification Document WEBUI-007 (16-webui-cost-monitoring.md)
 */

'use client';

import { BudgetIndicator } from '@/components/cost/budget-indicator';
import { CostBreakdownChart } from '@/components/cost/cost-breakdown-chart';
import { CostSummaryCard } from '@/components/cost/cost-summary-card';
import { TokenUsageTable } from '@/components/cost/token-usage-table';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue
} from '@/components/ui/select';
import { Skeleton } from '@/components/ui/skeleton';
import {
    useBudgetStatus,
    useCostHistory,
    useWorkspaceCostSummary,
} from '@/hooks';
import {
    Calendar, DollarSign,
    Download, RefreshCw,
    TrendingUp
} from 'lucide-react';
import { useState } from 'react';

type TimePeriod = '7d' | '30d' | '90d' | 'all';

export default function CostDashboardPage() {
  const [period, setPeriod] = useState<TimePeriod>('30d');
  
  // Fetch data
  const { data: summary, isLoading: isSummaryLoading, refetch: refetchSummary } = useWorkspaceCostSummary();
  const { data: budget, isLoading: isBudgetLoading } = useBudgetStatus();
  const { data: history, isLoading: isHistoryLoading } = useCostHistory({
    granularity: period === '7d' ? 'day' : period === '30d' ? 'day' : 'week',
  });

  const handleExport = (format: 'json' | 'csv') => {
    if (!summary) return;

    let content: string;
    let filename: string;
    let mimeType: string;

    if (format === 'json') {
      content = JSON.stringify({ summary, history }, null, 2);
      filename = `cost-report-${new Date().toISOString().split('T')[0]}.json`;
      mimeType = 'application/json';
    } else {
      // CSV export of history
      const headers = ['Date', 'Cost (USD)', 'Documents', 'Tokens'];
      const rows = history?.map(h => [
        h.timestamp,
        h.total_cost.toFixed(4),
        h.document_count.toString(),
        h.total_tokens.toString(),
      ]) ?? [];

      content = [
        headers.join(','),
        ...rows.map(row => row.join(',')),
      ].join('\n');
      filename = `cost-report-${new Date().toISOString().split('T')[0]}.csv`;
      mimeType = 'text/csv';
    }

    const blob = new Blob([content], { type: mimeType });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="border-b px-6 py-4">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-semibold flex items-center gap-2">
              <DollarSign className="h-6 w-6" />
              Cost Dashboard
            </h1>
            <p className="text-sm text-muted-foreground mt-1">
              Monitor LLM costs and usage across your workspace
            </p>
          </div>
          
          <div className="flex items-center gap-2">
            {/* Period selector */}
            <Select value={period} onValueChange={(v) => setPeriod(v as TimePeriod)}>
              <SelectTrigger className="w-32">
                <Calendar className="h-4 w-4 mr-2" />
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="7d">Last 7 days</SelectItem>
                <SelectItem value="30d">Last 30 days</SelectItem>
                <SelectItem value="90d">Last 90 days</SelectItem>
                <SelectItem value="all">All time</SelectItem>
              </SelectContent>
            </Select>

            {/* Refresh */}
            <Button
              variant="outline"
              size="icon"
              onClick={() => refetchSummary()}
            >
              <RefreshCw className="h-4 w-4" />
            </Button>

            {/* Export */}
            <Button
              variant="outline"
              onClick={() => handleExport('csv')}
            >
              <Download className="h-4 w-4 mr-2" />
              Export
            </Button>
          </div>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto p-6">
        <div className="max-w-7xl mx-auto space-y-6">
          {/* Top row: Summary and Budget */}
          <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
            <div className="lg:col-span-2">
              <CostSummaryCard
                summary={summary ?? null}
                isLoading={isSummaryLoading}
              />
            </div>
            <div>
              <BudgetIndicator
                budget={budget ?? null}
                status={null}
                alerts={[]}
                isLoading={isBudgetLoading}
              />
            </div>
          </div>

          {/* Charts row */}
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            {/* Cost by operation */}
            <CostBreakdownChart
              breakdown={summary ? {
                total_cost: summary.total_cost,
                by_stage: summary.by_operation?.map(op => ({
                  stage: op.operation,
                  cost: op.cost,
                  tokens: { 
                    input: op.input_tokens ?? 0, 
                    output: op.output_tokens ?? 0, 
                    total: op.total_tokens ?? 0 
                  },
                  call_count: op.call_count ?? 0,
                  cached_calls: 0,
                })) ?? [],
                tokens: { input: 0, output: 0, total: summary.total_tokens },
              } : null}
              type="bar"
              isLoading={isSummaryLoading}
            />

            {/* Cost trend chart */}
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-base flex items-center gap-2">
                  <TrendingUp className="h-4 w-4" />
                  Cost Trend
                </CardTitle>
              </CardHeader>
              <CardContent>
                {isHistoryLoading ? (
                  <Skeleton className="h-48 w-full" />
                ) : history && history.length > 0 ? (
                  <CostTrendChart data={history} />
                ) : (
                  <div className="h-48 flex items-center justify-center text-muted-foreground">
                    No historical data available
                  </div>
                )}
              </CardContent>
            </Card>
          </div>

          {/* Token usage table */}
          <TokenUsageTable
            stages={summary?.by_operation?.map(op => ({
              stage: op.operation,
              cost: op.cost,
              tokens: { 
                input: op.input_tokens ?? 0, 
                output: op.output_tokens ?? 0, 
                total: op.total_tokens ?? 0 
              },
              call_count: op.call_count ?? 0,
              cached_calls: 0,
            })) ?? null}
            isLoading={isSummaryLoading}
          />
        </div>
      </div>
    </div>
  );
}

/**
 * Simple cost trend chart using bars.
 */
function CostTrendChart({
  data,
}: {
  data: Array<{ timestamp: string; total_cost: number; document_count: number }>;
}) {
  const maxCost = Math.max(...data.map(d => d.total_cost), 0.01);

  return (
    <div className="h-48 flex items-end gap-1">
      {data.map((item, index) => {
        const height = (item.total_cost / maxCost) * 100;
        const date = new Date(item.timestamp);
        const label = date.toLocaleDateString(undefined, { 
          month: 'short', 
          day: 'numeric' 
        });

        return (
          <div
            key={index}
            className="flex-1 flex flex-col items-center gap-1"
            title={`${label}: $${item.total_cost.toFixed(4)} (${item.document_count} docs)`}
          >
            <div
              className="w-full bg-primary/80 hover:bg-primary rounded-t transition-all"
              style={{ height: `${Math.max(height, 2)}%` }}
            />
            {data.length <= 14 && (
              <span className="text-xs text-muted-foreground">
                {date.getDate()}
              </span>
            )}
          </div>
        );
      })}
    </div>
  );
}

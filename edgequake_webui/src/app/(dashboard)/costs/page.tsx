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
import { CostTrendChart } from '@/components/cost/cost-trend-chart';
import { PageHeader } from '@/components/shared/page-header';
import { PageShell } from '@/components/shared/page-shell';
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
import { formatCost } from '@/lib/format';
import type { BudgetInfo } from '@/types/cost';
import {
    Calendar,
    Download, RefreshCw,
    TrendingUp
} from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

type TimePeriod = '7d' | '30d' | '90d' | 'all';

function statusFromBudget(budget: BudgetInfo) {
  const limit = budget.monthly_budget_usd;
  const used = budget.spent_usd;
  return {
    current_usage_usd: used,
    limit_usd: limit,
    percentage_used: limit > 0 ? (used / limit) * 100 : 0,
    period: 'monthly' as const,
    reset_at: '',
    alert_triggered:
      budget.is_over_budget ||
      (limit > 0 && budget.alert_threshold > 0 && (used / limit) * 100 >= budget.alert_threshold),
  };
}

export default function CostDashboardPage() {
  const { t } = useTranslation();
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
        formatCost(h.total_cost).replace(/[^0-9.-]/g, '') || h.total_cost.toFixed(4),
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
    <div className="flex flex-col h-full overflow-auto">
      <PageShell>
        <PageHeader
          title={t('costs.title', 'Cost Dashboard')}
          description={t(
            'costs.subtitle',
            'Monitor LLM costs and usage across your workspace',
          )}
          actions={
            <>
              <Select value={period} onValueChange={(v) => setPeriod(v as TimePeriod)}>
                <SelectTrigger
                  className="w-44 whitespace-nowrap"
                  aria-label={t('costs.period.label', 'Time period')}
                >
                  <Calendar className="h-4 w-4 mr-2" />
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="7d">{t('costs.period.7d', 'Last 7 days')}</SelectItem>
                  <SelectItem value="30d">{t('costs.period.30d', 'Last 30 days')}</SelectItem>
                  <SelectItem value="90d">{t('costs.period.90d', 'Last 90 days')}</SelectItem>
                  <SelectItem value="all">{t('costs.period.all', 'All time')}</SelectItem>
                </SelectContent>
              </Select>
              <Button
                variant="outline"
                size="icon"
                aria-label={t('common.refresh', 'Refresh')}
                onClick={() => refetchSummary()}
              >
                <RefreshCw className="h-4 w-4" />
              </Button>
              <Button variant="outline" onClick={() => handleExport('csv')}>
                <Download className="h-4 w-4 mr-2" />
                {t('common.download', 'Export')}
              </Button>
            </>
          }
        />

        <div className="space-y-page">
          {/* Top row: Summary and Budget */}
          <div className="grid min-w-0 grid-cols-1 lg:grid-cols-3 gap-page [&>*]:min-w-0">
            <div className="lg:col-span-2">
              <CostSummaryCard
                summary={summary ?? null}
                isLoading={isSummaryLoading && !summary}
              />
            </div>
            <div>
              <BudgetIndicator
                budget={budget ?? null}
                status={budget ? statusFromBudget(budget) : null}
                alerts={[]}
                isLoading={isBudgetLoading && !budget}
              />
            </div>
          </div>

          {/* Charts row */}
          <div className="grid min-w-0 grid-cols-1 lg:grid-cols-2 gap-page [&>*]:min-w-0">
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
              isLoading={isSummaryLoading && !summary}
            />

            {/* Cost trend chart — SPEC-100: fixed h-48 for skeleton/empty/live */}
            <Card data-testid="spec100-costs-trend">
              <CardHeader className="pb-2">
                <CardTitle className="flex items-center gap-2">
                  <TrendingUp className="h-4 w-4" />
                  Cost Trend
                </CardTitle>
              </CardHeader>
              <CardContent className="min-h-48">
                {isHistoryLoading && !history ? (
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
            isLoading={isSummaryLoading && !summary}
          />
        </div>
      </PageShell>
    </div>
  );
}

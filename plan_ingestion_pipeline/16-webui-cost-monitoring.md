# WebUI Specification: Cost Monitoring

> Document ID: WEBUI-007
> Version: 1.0
> Created: 2024-12-28
> Status: SPECIFICATION

---

## Table of Contents

1. [Overview](#1-overview)
2. [Cost Data Model](#2-cost-data-model)
3. [UI Components](#3-ui-components)
4. [Cost Dashboard](#4-cost-dashboard)
5. [Budget Management](#5-budget-management)
6. [Real-Time Cost Updates](#6-real-time-cost-updates)
7. [Export & Reporting](#7-export--reporting)

---

## 1. Overview

### 1.1 Purpose

This document specifies the cost monitoring UI for EdgeQuake WebUI. It enables users to track LLM costs at multiple granularities: per-document, per-operation, per-tenant, and overall system usage.

### 1.2 Cost Visibility Levels

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         COST VISIBILITY HIERARCHY                           │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│  SYSTEM LEVEL                                                               │
│  └─ Total cost across all tenants                                          │
│     └─ Monthly/daily trends                                                │
└─────────────────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  TENANT LEVEL                                                               │
│  └─ Total cost for tenant                                                  │
│     └─ Budget status and alerts                                            │
│     └─ Cost by operation type                                              │
└─────────────────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  DOCUMENT LEVEL                                                             │
│  └─ Total cost for document ingestion                                      │
│     └─ Cost by stage (extract, glean, summarize, embed)                    │
│     └─ Token usage details                                                 │
└─────────────────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  CHUNK LEVEL                                                                │
│  └─ Cost for individual chunk extraction                                   │
│     └─ Prompt tokens / Completion tokens                                   │
│     └─ Cache hit savings                                                   │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 1.3 Requirements

| Requirement | Description |
|-------------|-------------|
| REQ-COST-001 | Display cost with $0.0001 precision |
| REQ-COST-002 | Real-time cost updates during ingestion |
| REQ-COST-003 | Cost breakdown by operation type |
| REQ-COST-004 | Budget thresholds with alerts |
| REQ-COST-005 | Historical cost trends (7d/30d/90d) |
| REQ-COST-006 | Export cost reports (CSV/JSON) |
| REQ-COST-007 | Show estimated vs actual costs |

---

## 2. Cost Data Model

### 2.1 API Response Types

```typescript
// Cost breakdown for a single document
interface DocumentCostBreakdown {
  document_id: string;
  document_name: string;
  total_cost_usd: number;
  token_usage: TokenUsage;
  stages: StageCostBreakdown[];
  estimated_cost_usd?: number;
  savings_from_cache_usd: number;
  ingested_at: string;
}

interface TokenUsage {
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
  embedding_tokens: number;
}

interface StageCostBreakdown {
  stage: IngestionStage;
  cost_usd: number;
  token_usage: TokenUsage;
  call_count: number;
  model: string;
  duration_ms: number;
  cached_calls: number;
}

// Aggregated cost summary
interface CostSummary {
  period: 'day' | 'week' | 'month' | 'all';
  start_date: string;
  end_date: string;
  total_cost_usd: number;
  document_count: number;
  average_cost_per_document: number;
  total_tokens: number;
  by_operation: OperationCost[];
  by_model: ModelCost[];
  daily_breakdown: DailyCost[];
}

interface OperationCost {
  operation: string;  // 'extraction', 'gleaning', 'summarization', 'embedding'
  cost_usd: number;
  percentage: number;
}

interface ModelCost {
  model: string;
  cost_usd: number;
  token_count: number;
  call_count: number;
}

interface DailyCost {
  date: string;
  cost_usd: number;
  document_count: number;
}

// Budget configuration
interface BudgetConfig {
  enabled: boolean;
  daily_limit_usd?: number;
  monthly_limit_usd?: number;
  alert_threshold_percent: number;  // e.g., 80
}

interface BudgetStatus {
  current_usage_usd: number;
  limit_usd: number;
  percentage_used: number;
  period: 'daily' | 'monthly';
  reset_at: string;
  alert_triggered: boolean;
}
```

### 2.2 Zustand Cost Store

```typescript
// src/lib/stores/use-cost-store.ts

import { create } from 'zustand';

interface CostState {
  // Real-time tracking during ingestion
  activeIngestionCosts: Map<string, number>;  // trackId -> cumulative cost
  
  // Budget status
  budgetStatus: BudgetStatus | null;
  
  // Actions
  updateIngestionCost: (trackId: string, cost: number) => void;
  clearIngestionCost: (trackId: string) => void;
  setBudgetStatus: (status: BudgetStatus) => void;
}

export const useCostStore = create<CostState>((set) => ({
  activeIngestionCosts: new Map(),
  budgetStatus: null,
  
  updateIngestionCost: (trackId, cost) => {
    set((state) => {
      const updated = new Map(state.activeIngestionCosts);
      updated.set(trackId, cost);
      return { activeIngestionCosts: updated };
    });
  },
  
  clearIngestionCost: (trackId) => {
    set((state) => {
      const updated = new Map(state.activeIngestionCosts);
      updated.delete(trackId);
      return { activeIngestionCosts: updated };
    });
  },
  
  setBudgetStatus: (status) => {
    set({ budgetStatus: status });
  },
}));
```

---

## 3. UI Components

### 3.1 CostBadge (Inline Display)

```tsx
// src/components/cost/cost-badge.tsx

interface CostBadgeProps {
  cost: number;
  estimated?: number;
  size?: 'sm' | 'md' | 'lg';
  showTooltip?: boolean;
  breakdown?: StageCostBreakdown[];
  className?: string;
}

export function CostBadge({
  cost,
  estimated,
  size = 'md',
  showTooltip = true,
  breakdown,
  className,
}: CostBadgeProps) {
  const formattedCost = formatCost(cost);
  const formattedEstimate = estimated ? formatCost(estimated) : null;
  
  const badge = (
    <Badge
      variant={cost > (estimated ?? cost) ? 'destructive' : 'secondary'}
      className={cn(
        size === 'sm' && 'text-xs px-1.5 py-0',
        size === 'md' && 'text-sm px-2 py-0.5',
        size === 'lg' && 'text-base px-3 py-1',
        className
      )}
    >
      💰 {formattedCost}
      {formattedEstimate && cost < estimated! && (
        <span className="text-muted-foreground ml-1">
          / {formattedEstimate}
        </span>
      )}
    </Badge>
  );
  
  if (!showTooltip || !breakdown) {
    return badge;
  }
  
  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>{badge}</TooltipTrigger>
        <TooltipContent className="p-0">
          <CostBreakdownTooltip breakdown={breakdown} />
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

function CostBreakdownTooltip({ breakdown }: { breakdown: StageCostBreakdown[] }) {
  return (
    <div className="p-3 space-y-2 min-w-48">
      <div className="font-medium text-sm">Cost Breakdown</div>
      <div className="space-y-1">
        {breakdown.map((stage) => (
          <div key={stage.stage} className="flex justify-between text-sm">
            <span className="capitalize">{stage.stage}</span>
            <span className="font-mono">{formatCost(stage.cost_usd)}</span>
          </div>
        ))}
      </div>
      <Separator />
      <div className="flex justify-between font-medium text-sm">
        <span>Total</span>
        <span className="font-mono">
          {formatCost(breakdown.reduce((sum, s) => sum + s.cost_usd, 0))}
        </span>
      </div>
    </div>
  );
}

function formatCost(usd: number): string {
  if (usd < 0.01) {
    return `$${usd.toFixed(4)}`;
  } else if (usd < 1) {
    return `$${usd.toFixed(3)}`;
  } else {
    return `$${usd.toFixed(2)}`;
  }
}
```

**Visual Examples:**

```
Size sm:  💰 $0.0045          (small badge, inline)
Size md:  💰 $0.0045          (standard)
Size lg:  💰 $0.0045 / $0.01  (with estimate, processing)

Over budget: 💰 $0.0150 (red background)

Tooltip hover:
┌──────────────────────────┐
│ Cost Breakdown           │
│ ─────────────────────── │
│ Extraction     $0.0040  │
│ Gleaning       $0.0004  │
│ Summarization  $0.0000  │
│ Embedding      $0.0001  │
│ ─────────────────────── │
│ Total          $0.0045  │
└──────────────────────────┘
```

### 3.2 CostSummaryCard

```tsx
// src/components/cost/cost-summary-card.tsx

interface CostSummaryCardProps {
  summary: CostSummary;
  className?: string;
}

export function CostSummaryCard({ summary, className }: CostSummaryCardProps) {
  return (
    <Card className={className}>
      <CardHeader className="pb-2">
        <CardTitle className="text-lg">Cost Summary</CardTitle>
        <CardDescription>
          {formatDateRange(summary.start_date, summary.end_date)}
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="grid grid-cols-2 gap-4">
          {/* Total Cost */}
          <div>
            <div className="text-2xl font-bold">
              ${summary.total_cost_usd.toFixed(2)}
            </div>
            <div className="text-xs text-muted-foreground">Total Cost</div>
          </div>
          
          {/* Documents Processed */}
          <div>
            <div className="text-2xl font-bold">
              {summary.document_count}
            </div>
            <div className="text-xs text-muted-foreground">Documents</div>
          </div>
          
          {/* Average Cost */}
          <div>
            <div className="text-lg font-medium">
              ${summary.average_cost_per_document.toFixed(4)}
            </div>
            <div className="text-xs text-muted-foreground">Avg per Document</div>
          </div>
          
          {/* Total Tokens */}
          <div>
            <div className="text-lg font-medium">
              {formatTokenCount(summary.total_tokens)}
            </div>
            <div className="text-xs text-muted-foreground">Tokens Used</div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
```

**Visual:**

```
┌────────────────────────────────────────────────────────────────┐
│ Cost Summary                                                   │
│ Dec 1 - Dec 28, 2024                                          │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  $15.42              142                                      │
│  Total Cost          Documents                                │
│                                                                │
│  $0.1086             2.4M                                     │
│  Avg per Document    Tokens Used                              │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

### 3.3 CostBreakdownChart

```tsx
// src/components/cost/cost-breakdown-chart.tsx

import { PieChart, Pie, Cell, ResponsiveContainer, Tooltip, Legend } from 'recharts';

interface CostBreakdownChartProps {
  breakdown: OperationCost[];
  height?: number;
  showLegend?: boolean;
  className?: string;
}

const COLORS = {
  extraction: '#3b82f6',    // blue
  gleaning: '#22c55e',      // green
  summarization: '#f59e0b', // amber
  embedding: '#8b5cf6',     // purple
};

export function CostBreakdownChart({
  breakdown,
  height = 200,
  showLegend = true,
  className,
}: CostBreakdownChartProps) {
  const data = breakdown.map(item => ({
    name: item.operation.charAt(0).toUpperCase() + item.operation.slice(1),
    value: item.cost_usd,
    percentage: item.percentage,
    color: COLORS[item.operation as keyof typeof COLORS] || '#9ca3af',
  }));
  
  return (
    <div className={cn('w-full', className)} style={{ height }}>
      <ResponsiveContainer width="100%" height="100%">
        <PieChart>
          <Pie
            data={data}
            cx="50%"
            cy="50%"
            innerRadius={40}
            outerRadius={70}
            paddingAngle={2}
            dataKey="value"
            label={({ name, percentage }) => `${name} ${percentage.toFixed(0)}%`}
          >
            {data.map((entry, index) => (
              <Cell key={`cell-${index}`} fill={entry.color} />
            ))}
          </Pie>
          <Tooltip
            formatter={(value: number) => [`$${value.toFixed(4)}`, 'Cost']}
          />
          {showLegend && <Legend />}
        </PieChart>
      </ResponsiveContainer>
    </div>
  );
}
```

### 3.4 TokenUsageTable

```tsx
// src/components/cost/token-usage-table.tsx

interface TokenUsageTableProps {
  stages: StageCostBreakdown[];
  className?: string;
}

export function TokenUsageTable({ stages, className }: TokenUsageTableProps) {
  const totals = useMemo(() => {
    return stages.reduce(
      (acc, stage) => ({
        prompt: acc.prompt + stage.token_usage.prompt_tokens,
        completion: acc.completion + stage.token_usage.completion_tokens,
        cost: acc.cost + stage.cost_usd,
        calls: acc.calls + stage.call_count,
        cached: acc.cached + stage.cached_calls,
      }),
      { prompt: 0, completion: 0, cost: 0, calls: 0, cached: 0 }
    );
  }, [stages]);
  
  return (
    <Table className={className}>
      <TableHeader>
        <TableRow>
          <TableHead>Stage</TableHead>
          <TableHead>Model</TableHead>
          <TableHead className="text-right">Prompt</TableHead>
          <TableHead className="text-right">Completion</TableHead>
          <TableHead className="text-right">Calls</TableHead>
          <TableHead className="text-right">Cached</TableHead>
          <TableHead className="text-right">Cost</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {stages.map((stage) => (
          <TableRow key={stage.stage}>
            <TableCell className="capitalize font-medium">{stage.stage}</TableCell>
            <TableCell className="font-mono text-xs">{stage.model}</TableCell>
            <TableCell className="text-right font-mono">
              {stage.token_usage.prompt_tokens.toLocaleString()}
            </TableCell>
            <TableCell className="text-right font-mono">
              {stage.token_usage.completion_tokens.toLocaleString()}
            </TableCell>
            <TableCell className="text-right">{stage.call_count}</TableCell>
            <TableCell className="text-right text-green-600">
              {stage.cached_calls > 0 && `${stage.cached_calls} ⚡`}
            </TableCell>
            <TableCell className="text-right font-mono">
              ${stage.cost_usd.toFixed(4)}
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
      <TableFooter>
        <TableRow>
          <TableCell colSpan={2} className="font-bold">Total</TableCell>
          <TableCell className="text-right font-mono font-bold">
            {totals.prompt.toLocaleString()}
          </TableCell>
          <TableCell className="text-right font-mono font-bold">
            {totals.completion.toLocaleString()}
          </TableCell>
          <TableCell className="text-right font-bold">{totals.calls}</TableCell>
          <TableCell className="text-right text-green-600 font-bold">
            {totals.cached > 0 && `${totals.cached} ⚡`}
          </TableCell>
          <TableCell className="text-right font-mono font-bold">
            ${totals.cost.toFixed(4)}
          </TableCell>
        </TableRow>
      </TableFooter>
    </Table>
  );
}
```

**Visual:**

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Stage         │ Model              │ Prompt   │ Compl.  │ Calls│Cache│ Cost │
├─────────────────────────────────────────────────────────────────────────────┤
│ Extraction    │ gpt-4o-mini        │  12,450  │  3,200  │  10  │ 2 ⚡│$0.040│
│ Gleaning      │ gpt-4o-mini        │   2,100  │    450  │   3  │    │$0.004│
│ Summarization │ gpt-4o-mini        │   1,800  │    320  │   1  │    │$0.003│
│ Embedding     │ text-embed-3-small │  15,000  │      0  │   1  │    │$0.001│
├─────────────────────────────────────────────────────────────────────────────┤
│ Total         │                    │  31,350  │  3,970  │  15  │ 2 ⚡│$0.048│
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.5 BudgetIndicator

```tsx
// src/components/cost/budget-indicator.tsx

interface BudgetIndicatorProps {
  status: BudgetStatus | null;
  variant?: 'compact' | 'full';
  className?: string;
}

export function BudgetIndicator({
  status,
  variant = 'compact',
  className,
}: BudgetIndicatorProps) {
  if (!status) return null;
  
  const percentUsed = status.percentage_used;
  const variant = percentUsed >= 100 ? 'destructive' : 
                  percentUsed >= 80 ? 'warning' : 'default';
  
  if (variant === 'compact') {
    return (
      <div className={cn('flex items-center gap-2', className)}>
        <Progress value={percentUsed} className="w-24 h-2" />
        <span className={cn(
          'text-sm font-mono',
          percentUsed >= 100 && 'text-destructive',
          percentUsed >= 80 && percentUsed < 100 && 'text-amber-500'
        )}>
          {percentUsed.toFixed(0)}%
        </span>
      </div>
    );
  }
  
  return (
    <Card className={cn(
      'border-2',
      percentUsed >= 100 && 'border-destructive',
      percentUsed >= 80 && percentUsed < 100 && 'border-amber-500',
      className
    )}>
      <CardHeader className="pb-2">
        <CardTitle className="text-base flex items-center gap-2">
          {status.period === 'daily' ? 'Daily Budget' : 'Monthly Budget'}
          {status.alert_triggered && (
            <Badge variant="destructive">Alert</Badge>
          )}
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          <Progress
            value={Math.min(percentUsed, 100)}
            className={cn(
              'h-3',
              percentUsed >= 100 && '[&>div]:bg-destructive',
              percentUsed >= 80 && percentUsed < 100 && '[&>div]:bg-amber-500'
            )}
          />
          <div className="flex justify-between text-sm">
            <span>
              ${status.current_usage_usd.toFixed(2)} / ${status.limit_usd.toFixed(2)}
            </span>
            <span className="text-muted-foreground">
              Resets {formatRelativeTime(status.reset_at)}
            </span>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
```

---

## 4. Cost Dashboard

### 4.1 Dashboard Layout

```tsx
// src/app/cost/page.tsx

export default function CostDashboardPage() {
  const [period, setPeriod] = useState<'day' | 'week' | 'month'>('week');
  
  const { data: summary, isLoading } = useCostSummary(period);
  const { data: budget } = useBudgetStatus();
  
  return (
    <div className="container mx-auto py-6 space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Cost Dashboard</h1>
        <div className="flex gap-2">
          <Select value={period} onValueChange={setPeriod}>
            <SelectTrigger className="w-32">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="day">Today</SelectItem>
              <SelectItem value="week">This Week</SelectItem>
              <SelectItem value="month">This Month</SelectItem>
            </SelectContent>
          </Select>
          <ExportCostReportButton period={period} />
        </div>
      </div>
      
      {/* Budget Alert */}
      {budget?.alert_triggered && (
        <Alert variant="destructive">
          <AlertCircle className="h-4 w-4" />
          <AlertTitle>Budget Alert</AlertTitle>
          <AlertDescription>
            You have used {budget.percentage_used.toFixed(0)}% of your {budget.period} budget.
          </AlertDescription>
        </Alert>
      )}
      
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Summary Card */}
        <CostSummaryCard summary={summary} className="lg:col-span-2" />
        
        {/* Budget Status */}
        <BudgetIndicator status={budget} variant="full" />
      </div>
      
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Cost by Operation */}
        <Card>
          <CardHeader>
            <CardTitle>Cost by Operation</CardTitle>
          </CardHeader>
          <CardContent>
            <CostBreakdownChart breakdown={summary?.by_operation ?? []} />
          </CardContent>
        </Card>
        
        {/* Daily Trend */}
        <Card>
          <CardHeader>
            <CardTitle>Daily Cost Trend</CardTitle>
          </CardHeader>
          <CardContent>
            <CostTrendChart data={summary?.daily_breakdown ?? []} />
          </CardContent>
        </Card>
      </div>
      
      {/* Recent Documents */}
      <Card>
        <CardHeader>
          <CardTitle>Recent Document Costs</CardTitle>
        </CardHeader>
        <CardContent>
          <RecentDocumentCostsTable />
        </CardContent>
      </Card>
    </div>
  );
}
```

### 4.2 Dashboard Visual

```
┌────────────────────────────────────────────────────────────────────────────┐
│ Cost Dashboard                                        [Week ▾] [📥 Export] │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│ ⚠️ Budget Alert: You have used 85% of your daily budget.                  │
│                                                                            │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│ ┌──────────────────────────────────────────────┐  ┌──────────────────────┐│
│ │ Cost Summary                                 │  │ Daily Budget         ││
│ │ Dec 22 - Dec 28, 2024                        │  │                      ││
│ │                                              │  │ ████████████████░░░  ││
│ │  $15.42              142                     │  │  85%                 ││
│ │  Total Cost          Documents               │  │                      ││
│ │                                              │  │ $8.50 / $10.00       ││
│ │  $0.1086             2.4M                    │  │ Resets in 6h         ││
│ │  Avg per Doc         Tokens Used             │  │                      ││
│ └──────────────────────────────────────────────┘  └──────────────────────┘│
│                                                                            │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│ ┌──────────────────────────────────────────────┐  ┌──────────────────────┐│
│ │ Cost by Operation                            │  │ Daily Cost Trend     ││
│ │                                              │  │                      ││
│ │         ┌────────────┐                       │  │     $2.5 ─┐          ││
│ │        /  Extraction \                       │  │          │  ┌──┐    ││
│ │       │     81%      │                       │  │     $2.0 ├──┤  ├──  ││
│ │        \  $12.50    /                        │  │          │  │  │    ││
│ │         └─────┬─────┘                        │  │     $1.5 ├──┤  │    ││
│ │   Gleaning    │     Embedding                │  │      Mon Tue Wed ... ││
│ │     12%       │        1%                    │  │                      ││
│ └──────────────────────────────────────────────┘  └──────────────────────┘│
│                                                                            │
├────────────────────────────────────────────────────────────────────────────┤
│ Recent Document Costs                                                      │
│ ─────────────────────────────────────────────────────────────────────────  │
│ Document               │ Status    │ Chunks │ Entities │ Cost    │ Time   │
│ ─────────────────────────────────────────────────────────────────────────  │
│ research-paper.pdf     │ ✓ Done    │   10   │    28    │ $0.0045 │ 2m ago │
│ quarterly-report.docx  │ ✓ Done    │   24   │    45    │ $0.0089 │ 1h ago │
│ meeting-notes.txt      │ ✓ Done    │    3   │    12    │ $0.0018 │ 3h ago │
│ ...                                                                        │
└────────────────────────────────────────────────────────────────────────────┘
```

### 4.3 Cost Trend Chart

```tsx
// src/components/cost/cost-trend-chart.tsx

import { LineChart, Line, XAxis, YAxis, Tooltip, ResponsiveContainer } from 'recharts';

interface CostTrendChartProps {
  data: DailyCost[];
  height?: number;
  showDocumentCount?: boolean;
  className?: string;
}

export function CostTrendChart({
  data,
  height = 200,
  showDocumentCount = false,
  className,
}: CostTrendChartProps) {
  const formattedData = data.map(d => ({
    ...d,
    date: format(new Date(d.date), 'MMM d'),
  }));
  
  return (
    <div className={cn('w-full', className)} style={{ height }}>
      <ResponsiveContainer width="100%" height="100%">
        <LineChart data={formattedData}>
          <XAxis dataKey="date" tick={{ fontSize: 12 }} />
          <YAxis 
            tick={{ fontSize: 12 }} 
            tickFormatter={(value) => `$${value.toFixed(2)}`}
          />
          <Tooltip
            formatter={(value: number, name: string) => [
              name === 'cost_usd' ? `$${value.toFixed(4)}` : value,
              name === 'cost_usd' ? 'Cost' : 'Documents'
            ]}
          />
          <Line
            type="monotone"
            dataKey="cost_usd"
            stroke="#3b82f6"
            strokeWidth={2}
            dot={{ r: 4 }}
          />
          {showDocumentCount && (
            <Line
              type="monotone"
              dataKey="document_count"
              stroke="#22c55e"
              strokeWidth={2}
              dot={{ r: 4 }}
              yAxisId="right"
            />
          )}
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}
```

---

## 5. Budget Management

### 5.1 Budget Settings Component

```tsx
// src/components/cost/budget-settings.tsx

interface BudgetSettingsProps {
  config: BudgetConfig;
  onSave: (config: BudgetConfig) => void;
}

export function BudgetSettings({ config, onSave }: BudgetSettingsProps) {
  const [enabled, setEnabled] = useState(config.enabled);
  const [dailyLimit, setDailyLimit] = useState(config.daily_limit_usd?.toString() ?? '');
  const [monthlyLimit, setMonthlyLimit] = useState(config.monthly_limit_usd?.toString() ?? '');
  const [alertThreshold, setAlertThreshold] = useState(config.alert_threshold_percent);
  
  const handleSave = () => {
    onSave({
      enabled,
      daily_limit_usd: dailyLimit ? parseFloat(dailyLimit) : undefined,
      monthly_limit_usd: monthlyLimit ? parseFloat(monthlyLimit) : undefined,
      alert_threshold_percent: alertThreshold,
    });
  };
  
  return (
    <Card>
      <CardHeader>
        <CardTitle>Budget Settings</CardTitle>
        <CardDescription>
          Configure spending limits and alerts
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex items-center justify-between">
          <Label htmlFor="budget-enabled">Enable Budget Limits</Label>
          <Switch
            id="budget-enabled"
            checked={enabled}
            onCheckedChange={setEnabled}
          />
        </div>
        
        {enabled && (
          <>
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="daily-limit">Daily Limit (USD)</Label>
                <Input
                  id="daily-limit"
                  type="number"
                  step="0.01"
                  placeholder="e.g., 10.00"
                  value={dailyLimit}
                  onChange={(e) => setDailyLimit(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="monthly-limit">Monthly Limit (USD)</Label>
                <Input
                  id="monthly-limit"
                  type="number"
                  step="1"
                  placeholder="e.g., 100.00"
                  value={monthlyLimit}
                  onChange={(e) => setMonthlyLimit(e.target.value)}
                />
              </div>
            </div>
            
            <div className="space-y-2">
              <Label htmlFor="alert-threshold">
                Alert Threshold ({alertThreshold}%)
              </Label>
              <Slider
                id="alert-threshold"
                min={50}
                max={100}
                step={5}
                value={[alertThreshold]}
                onValueChange={([value]) => setAlertThreshold(value)}
              />
              <p className="text-xs text-muted-foreground">
                You'll be notified when usage reaches this percentage of your limit.
              </p>
            </div>
          </>
        )}
        
        <Button onClick={handleSave}>Save Settings</Button>
      </CardContent>
    </Card>
  );
}
```

---

## 6. Real-Time Cost Updates

### 6.1 WebSocket Cost Events

Cost updates are received via WebSocket during ingestion:

```typescript
// Handle cost_update events from WebSocket

interface CostUpdate {
  type: 'cost_update';
  track_id: string;
  stage: IngestionStage;
  operation: string;
  cost_usd: number;
  tokens_used?: {
    input: number;
    output: number;
  };
  cumulative_cost_usd: number;
}

// In the ingestion store
updateProgress: (message) => {
  if (message.type === 'cost_update') {
    set((state) => {
      const track = state.tracks.get(message.track_id);
      if (track) {
        track.cost_usd = message.cumulative_cost_usd;
      }
    });
  }
};
```

### 6.2 Live Cost Display in Progress Panel

```tsx
// In IngestionProgressPanel

function LiveCostDisplay({ trackId }: { trackId: string }) {
  const tracks = useIngestionStore((state) => state.tracks);
  const track = tracks.get(trackId);
  
  const cost = track?.cost_usd ?? 0;
  const estimated = track?.estimated_cost_usd;
  
  return (
    <div className="flex items-center gap-2">
      <CostBadge cost={cost} estimated={estimated} size="md" />
      {cost > 0 && (
        <span className="text-xs text-muted-foreground animate-pulse">
          updating...
        </span>
      )}
    </div>
  );
}
```

---

## 7. Export & Reporting

### 7.1 Export Button Component

```tsx
// src/components/cost/export-cost-report-button.tsx

interface ExportCostReportButtonProps {
  period: 'day' | 'week' | 'month';
}

export function ExportCostReportButton({ period }: ExportCostReportButtonProps) {
  const [isExporting, setIsExporting] = useState(false);
  
  const handleExport = async (format: 'csv' | 'json') => {
    setIsExporting(true);
    try {
      const data = await getCostReport(period, format);
      
      const filename = `cost-report-${period}-${format(new Date(), 'yyyy-MM-dd')}.${format}`;
      downloadFile(data, filename, format === 'csv' ? 'text/csv' : 'application/json');
    } finally {
      setIsExporting(false);
    }
  };
  
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" disabled={isExporting}>
          <DownloadIcon className="h-4 w-4 mr-2" />
          {isExporting ? 'Exporting...' : 'Export'}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent>
        <DropdownMenuItem onClick={() => handleExport('csv')}>
          Export as CSV
        </DropdownMenuItem>
        <DropdownMenuItem onClick={() => handleExport('json')}>
          Export as JSON
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
```

### 7.2 Export Format Examples

**CSV Format:**
```csv
date,document_id,document_name,total_cost_usd,extraction_cost,gleaning_cost,embedding_cost,prompt_tokens,completion_tokens
2024-12-28,doc-1,research.pdf,0.0045,0.0040,0.0004,0.0001,12450,3200
2024-12-28,doc-2,report.docx,0.0089,0.0078,0.0008,0.0003,24100,5800
```

**JSON Format:**
```json
{
  "period": "week",
  "start_date": "2024-12-22",
  "end_date": "2024-12-28",
  "summary": {
    "total_cost_usd": 15.42,
    "document_count": 142,
    "average_cost_per_document": 0.1086
  },
  "documents": [
    {
      "document_id": "doc-1",
      "document_name": "research.pdf",
      "total_cost_usd": 0.0045,
      "stages": [...]
    }
  ]
}
```

---

## Appendix: React Query Hooks

```typescript
// src/lib/hooks/use-cost-queries.ts

export function useCostSummary(period: 'day' | 'week' | 'month') {
  return useQuery({
    queryKey: ['cost-summary', period],
    queryFn: () => getCostSummary(period),
    staleTime: 60000,  // 1 minute
    refetchInterval: 60000,
  });
}

export function useDocumentCost(documentId: string) {
  return useQuery({
    queryKey: ['document-cost', documentId],
    queryFn: () => getDocumentCost(documentId),
    enabled: !!documentId,
  });
}

export function useBudgetStatus() {
  return useQuery({
    queryKey: ['budget-status'],
    queryFn: getBudgetStatus,
    staleTime: 30000,  // 30 seconds
    refetchInterval: 30000,
  });
}

export function useBudgetConfigMutation() {
  const queryClient = useQueryClient();
  
  return useMutation({
    mutationFn: updateBudgetConfig,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['budget-status'] });
    },
  });
}
```

---

_End of Document WEBUI-007_

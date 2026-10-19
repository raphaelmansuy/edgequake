'use client';

/**
 * Daily cost bars. Each column is a fixed-height flex column so percentage bar
 * heights resolve (a percentage inside an auto-height flex item collapses to 0,
 * which rendered the chart blank — SPEC-168 C-1).
 */
import { useTranslation } from 'react-i18next';

export interface CostTrendPoint {
  timestamp: string;
  total_cost: number;
  document_count: number;
}

const formatUsd = (n: number) => `$${n < 1 ? n.toFixed(2) : n.toFixed(n < 10 ? 2 : 1)}`;

export function CostTrendChart({ data }: { data: CostTrendPoint[] }) {
  const { t } = useTranslation();
  const maxCost = Math.max(...data.map((d) => d.total_cost), 0.01);
  const total = data.reduce((sum, d) => sum + d.total_cost, 0);
  const showDayNumbers = data.length <= 14;

  return (
    <figure
      className="m-0"
      aria-label={t('costs.trendAria', 'Daily cost, {{count}} days, total {{total}}', {
        count: data.length,
        total: formatUsd(total),
      })}
    >
      <div className="flex h-48 gap-2">
        <div
          className="flex w-10 shrink-0 flex-col justify-between pb-5 text-right text-[11px] text-muted-foreground"
          aria-hidden="true"
        >
          <span>{formatUsd(maxCost)}</span>
          <span>$0</span>
        </div>
        <div className="flex min-w-0 flex-1 items-stretch gap-[3px] border-b border-l border-border/60 pl-1">
          {data.map((item, index) => {
            const date = new Date(item.timestamp);
            const label = date.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
            const pct = Math.max((item.total_cost / maxCost) * 100, 2);
            return (
              <div
                key={`${item.timestamp}-${index}`}
                className="group flex min-w-0 flex-1 flex-col"
                title={`${label}: $${item.total_cost.toFixed(4)} (${item.document_count} docs)`}
              >
                <div className="flex min-h-0 flex-1 items-end">
                  <div
                    className="w-full rounded-t bg-blue-500/70 transition-colors group-hover:bg-blue-500"
                    style={{ height: `${pct}%` }}
                  />
                </div>
                <span className="h-5 pt-1 text-center text-[11px] text-muted-foreground" aria-hidden="true">
                  {showDayNumbers ? date.getDate() : ''}
                </span>
              </div>
            );
          })}
        </div>
      </div>
    </figure>
  );
}

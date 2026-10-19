/**
 * @fileoverview Statistics card component for dashboard metrics display
 *
 * @implements FEAT1001 - Dashboard statistics visualization
 * @implements FEAT1002 - Trend indicators for metrics
 *
 * @see UC1101 - User views knowledge base statistics
 * @see UC1102 - User monitors entity/document counts
 *
 * @enforces BR1001 - Loading state with skeleton placeholders
 * @enforces BR1002 - Color-coded variant styling
 */
'use client';

import { Skeleton } from '@/components/ui/skeleton';
import { cn } from '@/lib/utils';
import type { LucideIcon } from 'lucide-react';
import { Minus, TrendingDown, TrendingUp } from 'lucide-react';
import Link from 'next/link';

export type StatsCardVariant = 'documents' | 'entities' | 'relationships' | 'types' | 'default';

interface StatsCardProps {
  title: string;
  value: number | string;
  description?: string;
  icon: LucideIcon;
  trend?: {
    value: number;
    isPositive: boolean;
  };
  isLoading?: boolean;
  /** P-G18: stats may be served from stale cache under ingestion load. */
  isStale?: boolean;
  className?: string;
  variant?: StatsCardVariant;
  /** LS-03: Optional hint shown when value is 0 — guides new users. */
  zeroHint?: string;
  /** Makes the whole card a link to where the number comes from. */
  href?: string;
}

const variantStyles: Record<StatsCardVariant, string> = {
  documents: 'stats-card-documents',
  entities: 'stats-card-entities',
  relationships: 'stats-card-relationships',
  types: 'stats-card-types',
  default: '',
};

const variantIconBg: Record<StatsCardVariant, string> = {
  documents: 'bg-blue-500/10',
  entities: 'bg-emerald-500/10',
  relationships: 'bg-violet-500/10',
  types: 'bg-orange-500/10',
  default: 'bg-primary/10',
};

const variantIconColor: Record<StatsCardVariant, string> = {
  documents: 'text-blue-600 dark:text-blue-400',
  entities: 'text-green-600 dark:text-green-400',
  relationships: 'text-purple-600 dark:text-purple-400',
  types: 'text-orange-600 dark:text-orange-400',
  default: 'text-primary',
};

/** Borderless cell: lives inside <StatStrip>, which draws the frame and dividers. */
const CELL = 'flex h-full min-w-0 flex-col bg-card px-4 py-3.5';

export function StatsCard({
  title,
  value,
  description,
  icon: Icon,
  trend,
  isLoading,
  isStale,
  className,
  variant = 'default',
  zeroHint,
  href,
}: StatsCardProps) {
  if (isLoading) {
    return (
      <div className={cn(CELL, className)} aria-busy="true">
        <div className="flex items-center gap-2">
          <Skeleton className="h-6 w-6 rounded-md" />
          <Skeleton className="h-3.5 w-20" />
        </div>
        <Skeleton className="mt-2.5 h-8 w-16" />
        <Skeleton className="mt-2 h-3 w-28" />
      </div>
    );
  }

  const TrendIcon = trend?.isPositive ? TrendingUp : trend?.value === 0 ? Minus : TrendingDown;

  const card = (
    <div
      data-testid="stats-card"
      data-variant={variant}
      data-value={value}
      className={cn(CELL, 'transition-colors duration-150', href && 'group-hover:bg-muted/40', variantStyles[variant], className)}
    >
      <div className="flex items-center gap-2">
        <span
          className={cn('flex h-6 w-6 shrink-0 items-center justify-center rounded-md', variantIconBg[variant])}
          aria-hidden="true"
        >
          <Icon className={cn('h-3.5 w-3.5', variantIconColor[variant])} />
        </span>
        <p className="min-w-0 truncate text-[0.8125rem] font-medium text-muted-foreground">
          {title}
          {isStale && (
            <span
              className="ml-1.5 text-xs font-normal text-amber-600 dark:text-amber-400"
              data-testid="stats-stale-badge"
            >
              (updating)
            </span>
          )}
        </p>
      </div>
      <div className="mt-2 flex items-baseline gap-2">
        <p className="text-[1.75rem] font-semibold leading-none tracking-tight tabular-nums" data-testid="stats-value">
          {typeof value === 'number' ? value.toLocaleString() : value}
        </p>
        {trend && trend.value !== 0 && (
          <div
            className={cn(
              'flex items-center gap-1 rounded-md px-1.5 py-0.5 text-xs font-medium',
              trend.isPositive
                ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400'
                : 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400'
            )}
          >
            <TrendIcon className="h-3 w-3" />
            {trend.isPositive ? '+' : ''}{trend.value}%
          </div>
        )}
      </div>
      {/* LS-03: zero hint guides new users; otherwise a one-line caption. */}
      {value === 0 && zeroHint ? (
        <p className="mt-1.5 line-clamp-2 text-xs text-muted-foreground" title={zeroHint}>{zeroHint}</p>
      ) : description ? (
        <p className="mt-1.5 line-clamp-2 text-xs text-muted-foreground" title={description}>{description}</p>
      ) : null}
    </div>
  );

  if (!href) return card;
  return (
    <Link
      href={href}
      aria-label={`${title}: ${typeof value === 'number' ? value.toLocaleString() : value}`}
      className="group block h-full focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring"
    >
      {card}
    </Link>
  );
}

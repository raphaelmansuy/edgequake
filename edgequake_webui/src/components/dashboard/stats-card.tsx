'use client';

import { Card, CardContent } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { cn } from '@/lib/utils';
import type { LucideIcon } from 'lucide-react';
import { Minus, TrendingDown, TrendingUp } from 'lucide-react';

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
  className?: string;
  variant?: StatsCardVariant;
}

const variantStyles: Record<StatsCardVariant, string> = {
  documents: 'stats-card-documents',
  entities: 'stats-card-entities',
  relationships: 'stats-card-relationships',
  types: 'stats-card-types',
  default: '',
};

const variantIconBg: Record<StatsCardVariant, string> = {
  documents: 'bg-blue-100 dark:bg-blue-900/30',
  entities: 'bg-green-100 dark:bg-green-900/30',
  relationships: 'bg-purple-100 dark:bg-purple-900/30',
  types: 'bg-orange-100 dark:bg-orange-900/30',
  default: 'bg-primary/10',
};

const variantIconColor: Record<StatsCardVariant, string> = {
  documents: 'text-blue-600 dark:text-blue-400',
  entities: 'text-green-600 dark:text-green-400',
  relationships: 'text-purple-600 dark:text-purple-400',
  types: 'text-orange-600 dark:text-orange-400',
  default: 'text-primary',
};

export function StatsCard({
  title,
  value,
  description,
  icon: Icon,
  trend,
  isLoading,
  className,
  variant = 'default',
}: StatsCardProps) {
  if (isLoading) {
    return (
      <Card className={cn('relative overflow-hidden', className)}>
        <CardContent className="p-6">
          <div className="flex items-center justify-between gap-4">
            <div className="space-y-3 flex-1">
              <Skeleton className="h-4 w-24" />
              <Skeleton className="h-9 w-20" />
              {description && <Skeleton className="h-3.5 w-36" />}
            </div>
            <Skeleton className="h-14 w-14 rounded-xl flex-shrink-0" />
          </div>
        </CardContent>
      </Card>
    );
  }

  const TrendIcon = trend?.isPositive ? TrendingUp : trend?.value === 0 ? Minus : TrendingDown;

  return (
    <Card 
      className={cn(
        'relative overflow-hidden transition-all duration-200',
        'hover:shadow-md hover:-translate-y-0.5',
        'focus-within:ring-2 focus-within:ring-ring focus-within:ring-offset-2',
        variantStyles[variant],
        className
      )}
    >
      <CardContent className="p-6">
        <div className="flex items-center justify-between gap-4">
          <div className="space-y-1.5 min-w-0 flex-1">
            <p className="text-sm font-medium text-muted-foreground truncate">
              {title}
            </p>
            <div className="flex items-baseline gap-3">
              <p className="text-3xl font-bold tracking-tight tabular-nums">
                {typeof value === 'number' ? value.toLocaleString() : value}
              </p>
              {trend && trend.value !== 0 && (
                <div 
                  className={cn(
                    'flex items-center gap-1 text-xs font-medium px-1.5 py-0.5 rounded-md',
                    trend.isPositive 
                      ? 'text-green-700 bg-green-100 dark:text-green-400 dark:bg-green-900/30' 
                      : 'text-red-700 bg-red-100 dark:text-red-400 dark:bg-red-900/30'
                  )}
                >
                  <TrendIcon className="h-3 w-3" />
                  {trend.isPositive ? '+' : ''}{trend.value}%
                </div>
              )}
            </div>
            {description && (
              <p className="text-sm text-muted-foreground truncate">
                {description}
              </p>
            )}
          </div>
          <div 
            className={cn(
              'flex h-14 w-14 items-center justify-center rounded-xl flex-shrink-0',
              'transition-transform duration-200 group-hover:scale-105',
              variantIconBg[variant]
            )}
          >
            <Icon className={cn('h-7 w-7', variantIconColor[variant])} />
          </div>
        </div>
      </CardContent>
      {/* Decorative gradient - only show for non-default variants */}
      {variant !== 'default' && (
        <div className="absolute bottom-0 left-0 right-0 h-1 bg-gradient-to-r from-transparent via-current to-transparent opacity-20" />
      )}
    </Card>
  );
}

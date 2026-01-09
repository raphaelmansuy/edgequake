/**
 * Budget Indicator Component
 * 
 * Visual indicator for budget status and limits.
 * Based on WebUI Specification Document WEBUI-007 (16-webui-cost-monitoring.md)
 *
 * @implements FEAT1040 - Budget status visualization
 * @implements FEAT1041 - Budget threshold alerts
 *
 * @see UC1201 - User monitors API budget usage
 * @see UC1202 - User receives budget threshold warnings
 *
 * @enforces BR1040 - Visual progress bar with color coding
 * @enforces BR1041 - Alert display for budget warnings
 */

'use client';

import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
} from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import { Skeleton } from '@/components/ui/skeleton';
import { cn } from '@/lib/utils';
import type { BudgetAlert, BudgetInfo, BudgetStatus } from '@/types/cost';
import {
    AlertCircle,
    AlertTriangle,
    CheckCircle,
    Clock,
    Wallet
} from 'lucide-react';

interface BudgetIndicatorProps {
  /** Budget info */
  budget: BudgetInfo | null;
  /** Budget status */
  status: BudgetStatus | null;
  /** Budget alerts */
  alerts?: BudgetAlert[];
  /** Loading state */
  isLoading?: boolean;
  /** Compact mode for inline display */
  compact?: boolean;
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
 * Formats remaining time until reset.
 */
function formatTimeUntilReset(resetAt?: string): string {
  if (!resetAt) return '';
  
  const reset = new Date(resetAt);
  const now = new Date();
  const diff = reset.getTime() - now.getTime();
  
  if (diff < 0) return 'Resetting...';
  
  const hours = Math.floor(diff / (1000 * 60 * 60));
  const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60));
  
  if (hours > 24) {
    const days = Math.floor(hours / 24);
    return `${days}d ${hours % 24}h`;
  }
  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  }
  return `${minutes}m`;
}

/**
 * Get status color based on percentage used.
 */
function getStatusColor(percentage: number): {
  variant: 'default' | 'warning' | 'destructive';
  color: string;
} {
  if (percentage >= 100) {
    return { variant: 'destructive', color: 'text-red-600' };
  }
  if (percentage >= 80) {
    return { variant: 'warning', color: 'text-yellow-600' };
  }
  return { variant: 'default', color: 'text-green-600' };
}

/**
 * Displays budget status with progress and alerts.
 */
export function BudgetIndicator({
  budget,
  status,
  alerts = [],
  isLoading = false,
  compact = false,
  className,
}: BudgetIndicatorProps) {
  if (isLoading) {
    return compact ? (
      <Skeleton className="h-6 w-24" />
    ) : (
      <Card className={className}>
        <CardHeader className="pb-2">
          <Skeleton className="h-5 w-32" />
        </CardHeader>
        <CardContent>
          <Skeleton className="h-16 w-full" />
        </CardContent>
      </Card>
    );
  }

  // No budget configured
  if (!budget || !status) {
    return compact ? null : (
      <Card className={className}>
        <CardContent className="py-8 text-center text-muted-foreground">
          <Wallet className="h-8 w-8 mx-auto mb-2 opacity-50" />
          <p>No budget configured</p>
        </CardContent>
      </Card>
    );
  }

  const percentage = status.percentage_used;
  const { variant, color } = getStatusColor(percentage);
  
  // Map variant to valid badge variants (warning -> secondary)
  const badgeVariant = variant === 'warning' ? 'secondary' : variant;

  // Compact mode for header/inline display
  if (compact) {
    return (
      <div className={cn('flex items-center gap-2', className)}>
        <Wallet className="h-4 w-4 text-muted-foreground" />
        <span className="text-sm">
          <span className={cn('font-medium', color)}>
            {formatCost(status.current_usage_usd)}
          </span>
          <span className="text-muted-foreground">
            {' / '}
            {formatCost(status.limit_usd)}
          </span>
        </span>
        {percentage >= 80 && (
          <Badge variant={badgeVariant} className="text-xs">
            {Math.round(percentage)}%
          </Badge>
        )}
      </div>
    );
  }

  return (
    <Card className={className}>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-base flex items-center gap-2">
            <Wallet className="h-4 w-4" />
            Budget Status
          </CardTitle>
          <Badge variant={badgeVariant}>
            {status.period === 'daily' ? 'Daily' : 'Monthly'}
          </Badge>
        </div>
        <CardDescription className="flex items-center gap-1.5">
          <Clock className="h-3 w-3" />
          Resets in {formatTimeUntilReset(status.reset_at)}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Progress bar */}
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-2xl font-bold font-mono">
              {formatCost(status.current_usage_usd)}
            </span>
            <span className="text-muted-foreground">
              of {formatCost(status.limit_usd)}
            </span>
          </div>
          <Progress
            value={Math.min(percentage, 100)}
            className={cn(
              'h-3',
              percentage >= 100 && '[&>div]:bg-red-500',
              percentage >= 80 && percentage < 100 && '[&>div]:bg-yellow-500'
            )}
          />
          <div className="flex items-center justify-between text-xs text-muted-foreground">
            <span>{Math.round(percentage)}% used</span>
            <span>
              {formatCost(Math.max(status.limit_usd - status.current_usage_usd, 0))} remaining
            </span>
          </div>
        </div>

        {/* Alerts */}
        {alerts.length > 0 && (
          <div className="space-y-2">
            {alerts.map((alert, index) => (
              <BudgetAlertItem key={index} alert={alert} />
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

/**
 * Individual budget alert item.
 */
function BudgetAlertItem({ alert }: { alert: BudgetAlert }) {
  const Icon = alert.type === 'critical' 
    ? AlertCircle 
    : alert.type === 'warning' 
      ? AlertTriangle 
      : CheckCircle;

  const variant = alert.type === 'critical'
    ? 'destructive'
    : 'default';

  return (
    <Alert variant={variant}>
      <Icon className="h-4 w-4" />
      <AlertTitle className="text-sm">Budget Alert</AlertTitle>
      <AlertDescription className="text-xs">
        {alert.message}
      </AlertDescription>
    </Alert>
  );
}

/**
 * Inline budget progress for minimal display.
 */
export function BudgetProgressInline({
  current,
  limit,
  className,
}: {
  current: number;
  limit: number;
  className?: string;
}) {
  const percentage = limit > 0 ? (current / limit) * 100 : 0;
  const { color } = getStatusColor(percentage);

  return (
    <div className={cn('flex items-center gap-2', className)}>
      <Progress
        value={Math.min(percentage, 100)}
        className="h-2 w-20"
      />
      <span className={cn('text-xs font-mono', color)}>
        {Math.round(percentage)}%
      </span>
    </div>
  );
}

export default BudgetIndicator;

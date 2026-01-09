/**
 * ETA Display Component
 * 
 * Shows estimated time remaining for ingestion.
 * Based on WebUI Specification Document WEBUI-004 (13-webui-components.md)
 *
 * @implements FEAT1062 - Estimated time remaining display
 * @implements FEAT1063 - Elapsed time tracking
 *
 * @see UC1403 - User sees estimated completion time
 * @see UC1404 - User monitors elapsed processing time
 *
 * @enforces BR1062 - Dynamic ETA calculation from progress
 * @enforces BR1063 - Human-readable time formatting
 */

'use client';

import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { cn } from '@/lib/utils';
import { CheckCircle, Clock, Timer } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';

interface EtaDisplayProps {
  /** Start timestamp (ISO string or Date) */
  startedAt?: string | Date;
  /** Estimated duration in milliseconds */
  estimatedDurationMs?: number;
  /** Current progress percentage (0-100) */
  progress: number;
  /** Whether the process is completed */
  isComplete?: boolean;
  /** Show elapsed time alongside ETA */
  showElapsed?: boolean;
  /** Size variant */
  size?: 'sm' | 'default' | 'lg';
  /** Custom class name */
  className?: string;
}

/**
 * Formats milliseconds into human-readable time.
 */
function formatTime(ms: number): string {
  if (ms < 0) return '--';
  
  const seconds = Math.floor(ms / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);

  if (hours > 0) {
    return `${hours}h ${minutes % 60}m`;
  }
  if (minutes > 0) {
    return `${minutes}m ${seconds % 60}s`;
  }
  return `${seconds}s`;
}

/**
 * Formats a short time string (e.g., "~45s")
 */
function formatShortTime(ms: number): string {
  if (ms < 0) return '--';
  
  const seconds = Math.floor(ms / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);

  if (hours > 0) {
    return `~${hours}h`;
  }
  if (minutes > 0) {
    return `~${minutes}m`;
  }
  return `~${seconds}s`;
}

/**
 * Displays estimated time remaining for ingestion.
 * 
 * Features:
 * - Real-time countdown
 * - Progress-based estimation
 * - Elapsed time display
 */
export function EtaDisplay({
  startedAt,
  estimatedDurationMs,
  progress,
  isComplete = false,
  showElapsed = false,
  size = 'default',
  className,
}: EtaDisplayProps) {
  const [elapsed, setElapsed] = useState(0);

  // Calculate elapsed time
  useEffect(() => {
    if (!startedAt || isComplete) return;

    const start = typeof startedAt === 'string' ? new Date(startedAt) : startedAt;
    
    const updateElapsed = () => {
      const now = new Date();
      setElapsed(now.getTime() - start.getTime());
    };

    updateElapsed();
    const interval = setInterval(updateElapsed, 1000);

    return () => clearInterval(interval);
  }, [startedAt, isComplete]);

  // Calculate ETA based on progress and elapsed time
  const eta = useMemo(() => {
    if (isComplete || progress >= 100) return 0;
    if (progress <= 0) return estimatedDurationMs || -1;

    // Calculate based on current rate of progress
    const rate = elapsed / progress; // ms per percent
    const remainingPercent = 100 - progress;
    const estimatedRemaining = rate * remainingPercent;

    // Blend with original estimate if available
    if (estimatedDurationMs) {
      const originalRemaining = estimatedDurationMs - elapsed;
      // Weight current estimate more as progress increases
      const weight = progress / 100;
      return originalRemaining * (1 - weight) + estimatedRemaining * weight;
    }

    return estimatedRemaining;
  }, [elapsed, progress, estimatedDurationMs, isComplete]);

  const iconSize = {
    sm: 'h-3 w-3',
    default: 'h-4 w-4',
    lg: 'h-5 w-5',
  };

  const textSize = {
    sm: 'text-xs',
    default: 'text-sm',
    lg: 'text-base',
  };

  // Completed state
  if (isComplete || progress >= 100) {
    return (
      <Tooltip>
        <TooltipTrigger asChild>
          <div className={cn('flex items-center gap-1.5', textSize[size], className)}>
            <CheckCircle className={cn(iconSize[size], 'text-green-500')} />
            <span className="text-green-600 dark:text-green-400 font-medium">
              Done
            </span>
            {showElapsed && startedAt && (
              <span className="text-muted-foreground">
                in {formatTime(elapsed)}
              </span>
            )}
          </div>
        </TooltipTrigger>
        <TooltipContent>
          <p>Completed in {formatTime(elapsed)}</p>
        </TooltipContent>
      </Tooltip>
    );
  }

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <div className={cn('flex items-center gap-1.5', textSize[size], className)}>
          {/* ETA indicator */}
          <Clock className={cn(iconSize[size], 'text-muted-foreground')} />
          <span className="text-muted-foreground">
            ETA: <span className="font-medium text-foreground">{formatShortTime(eta)}</span>
          </span>
          
          {/* Elapsed time */}
          {showElapsed && (
            <>
              <span className="text-muted-foreground/50 mx-1">|</span>
              <Timer className={cn(iconSize[size], 'text-muted-foreground/70')} />
              <span className="text-muted-foreground">
                {formatTime(elapsed)}
              </span>
            </>
          )}
        </div>
      </TooltipTrigger>
      <TooltipContent>
        <div className="space-y-1">
          <p>Estimated remaining: {formatTime(eta)}</p>
          <p>Elapsed: {formatTime(elapsed)}</p>
          <p>Progress: {Math.round(progress)}%</p>
        </div>
      </TooltipContent>
    </Tooltip>
  );
}

/**
 * Compact ETA display for inline use.
 */
export function EtaInline({
  progress,
  estimatedDurationMs,
  className,
}: {
  progress: number;
  estimatedDurationMs?: number;
  className?: string;
}) {
  // Simple estimation
  const eta = useMemo(() => {
    if (progress >= 100 || progress <= 0) return -1;
    if (!estimatedDurationMs) return -1;
    
    const remaining = estimatedDurationMs * (1 - progress / 100);
    return remaining;
  }, [progress, estimatedDurationMs]);

  if (eta < 0) return null;

  return (
    <span className={cn('text-xs text-muted-foreground', className)}>
      {formatShortTime(eta)}
    </span>
  );
}

export default EtaDisplay;

/**
 * Table Skeleton Component
 * 
 * Shows a loading skeleton while a markdown table is being streamed.
 * Displays shimmer animation to indicate content is loading.
 */
'use client';

import { cn } from '@/lib/utils';
import { memo } from 'react';

interface TableSkeletonProps {
  /** Number of rows to show in skeleton */
  rows?: number;
  /** Number of columns to show */
  columns?: number;
  /** Additional CSS classes */
  className?: string;
}

/**
 * Shimmer animation skeleton for tables during streaming
 */
export const TableSkeleton = memo(function TableSkeleton({
  rows = 3,
  columns = 4,
  className,
}: TableSkeletonProps) {
  return (
    <div 
      className={cn(
        'my-4 overflow-hidden rounded-lg border border-zinc-700',
        className
      )}
      role="status"
      aria-label="Loading table"
    >
      {/* Header row */}
      <div className="flex bg-zinc-800/50 border-b border-zinc-700">
        {Array.from({ length: columns }).map((_, i) => (
          <div 
            key={`header-${i}`}
            className="flex-1 px-4 py-3"
          >
            <div 
              className={cn(
                'h-4 rounded bg-zinc-700',
                // Varying widths for natural look
                i === 0 && 'w-24',
                i === 1 && 'w-32',
                i === 2 && 'w-28',
                i >= 3 && 'w-20',
              )}
            />
          </div>
        ))}
      </div>
      
      {/* Data rows */}
      {Array.from({ length: rows }).map((_, rowIndex) => (
        <div 
          key={`row-${rowIndex}`}
          className={cn(
            'flex border-b border-zinc-800 last:border-b-0',
            rowIndex % 2 === 0 ? 'bg-transparent' : 'bg-zinc-800/20'
          )}
        >
          {Array.from({ length: columns }).map((_, colIndex) => (
            <div 
              key={`cell-${rowIndex}-${colIndex}`}
              className="flex-1 px-4 py-2.5"
            >
              <div 
                className={cn(
                  'h-3.5 rounded bg-zinc-800',
                  'relative overflow-hidden',
                  // Random widths for natural appearance
                  (rowIndex + colIndex) % 3 === 0 && 'w-3/4',
                  (rowIndex + colIndex) % 3 === 1 && 'w-1/2',
                  (rowIndex + colIndex) % 3 === 2 && 'w-2/3',
                )}
              >
              </div>
            </div>
          ))}
        </div>
      ))}
      
      {/* Loading indicator */}
      <div className="flex items-center justify-center gap-2 py-2 text-xs text-zinc-500">
        <div className="flex gap-1">
          <span 
            className="w-1.5 h-1.5 rounded-full bg-zinc-600 animate-bounce"
            style={{ animationDelay: '0ms' }}
          />
          <span 
            className="w-1.5 h-1.5 rounded-full bg-zinc-600 animate-bounce"
            style={{ animationDelay: '150ms' }}
          />
          <span 
            className="w-1.5 h-1.5 rounded-full bg-zinc-600 animate-bounce"
            style={{ animationDelay: '300ms' }}
          />
        </div>
        <span>Loading table...</span>
      </div>
    </div>
  );
});

export default TableSkeleton;

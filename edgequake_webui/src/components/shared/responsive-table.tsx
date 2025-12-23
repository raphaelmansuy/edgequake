'use client';

import { useMediaQuery } from '@/hooks/use-media-query';
import { cn } from '@/lib/utils';
import { ReactNode } from 'react';

/**
 * Column configuration for the responsive table
 */
interface Column<T> {
  key: keyof T;
  header: string;
  render?: (value: T[keyof T], item: T) => ReactNode;
  className?: string;
  hideOnMobile?: boolean;
}

/**
 * Props for the responsive table component
 */
interface ResponsiveTableProps<T extends { id: string }> {
  data: T[];
  columns: Column<T>[];
  onRowClick?: (item: T) => void;
  renderActions?: (item: T) => ReactNode;
  isLoading?: boolean;
  emptyMessage?: string;
  className?: string;
}

/**
 * A responsive table component that displays as cards on mobile devices
 * and a traditional table on desktop.
 */
export function ResponsiveTable<T extends { id: string }>({
  data,
  columns,
  onRowClick,
  renderActions,
  isLoading,
  emptyMessage = 'No items found',
  className,
}: ResponsiveTableProps<T>) {
  const isMobile = useMediaQuery('(max-width: 768px)');

  if (isLoading) {
    return (
      <div className="space-y-3">
        {Array.from({ length: 3 }).map((_, i) => (
          <div
            key={i}
            className="bg-card border rounded-lg p-4 animate-pulse"
          >
            <div className="h-4 bg-muted rounded w-3/4 mb-2" />
            <div className="h-3 bg-muted rounded w-1/2" />
          </div>
        ))}
      </div>
    );
  }

  if (data.length === 0) {
    return (
      <div className="text-center py-8 text-muted-foreground">
        {emptyMessage}
      </div>
    );
  }

  // Mobile card layout
  if (isMobile) {
    return (
      <div className={cn('space-y-3', className)}>
        {data.map((item) => (
          <div
            key={item.id}
            className={cn(
              'bg-card border rounded-lg p-4 space-y-2',
              onRowClick && 'cursor-pointer hover:bg-accent/50 transition-colors'
            )}
            onClick={() => onRowClick?.(item)}
            role={onRowClick ? 'button' : undefined}
            tabIndex={onRowClick ? 0 : undefined}
            onKeyDown={(e) => {
              if (onRowClick && (e.key === 'Enter' || e.key === ' ')) {
                e.preventDefault();
                onRowClick(item);
              }
            }}
          >
            {columns
              .filter((col) => !col.hideOnMobile)
              .map((col) => (
                <div
                  key={String(col.key)}
                  className={cn('flex justify-between items-center', col.className)}
                >
                  <span className="text-sm text-muted-foreground">
                    {col.header}
                  </span>
                  <span className="text-sm font-medium text-right">
                    {col.render
                      ? col.render(item[col.key], item)
                      : String(item[col.key] ?? '')}
                  </span>
                </div>
              ))}
            {renderActions && (
              <div className="pt-2 border-t flex justify-end gap-2">
                {renderActions(item)}
              </div>
            )}
          </div>
        ))}
      </div>
    );
  }

  // Desktop table layout
  return (
    <div className={cn('overflow-x-auto', className)}>
      <table className="w-full">
        <thead>
          <tr className="border-b bg-muted/50">
            {columns.map((col) => (
              <th
                key={String(col.key)}
                className={cn(
                  'px-4 py-3 text-left text-sm font-medium text-muted-foreground',
                  col.className
                )}
              >
                {col.header}
              </th>
            ))}
            {renderActions && (
              <th className="px-4 py-3 text-right text-sm font-medium text-muted-foreground">
                Actions
              </th>
            )}
          </tr>
        </thead>
        <tbody>
          {data.map((item) => (
            <tr
              key={item.id}
              className={cn(
                'border-b hover:bg-muted/30 transition-colors',
                onRowClick && 'cursor-pointer'
              )}
              onClick={() => onRowClick?.(item)}
              role={onRowClick ? 'button' : undefined}
              tabIndex={onRowClick ? 0 : undefined}
              onKeyDown={(e) => {
                if (onRowClick && (e.key === 'Enter' || e.key === ' ')) {
                  e.preventDefault();
                  onRowClick(item);
                }
              }}
            >
              {columns.map((col) => (
                <td
                  key={String(col.key)}
                  className={cn('px-4 py-3 text-sm', col.className)}
                >
                  {col.render
                    ? col.render(item[col.key], item)
                    : String(item[col.key] ?? '')}
                </td>
              ))}
              {renderActions && (
                <td className="px-4 py-3 text-right">
                  <div className="flex justify-end gap-2">
                    {renderActions(item)}
                  </div>
                </td>
              )}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export type { Column, ResponsiveTableProps };

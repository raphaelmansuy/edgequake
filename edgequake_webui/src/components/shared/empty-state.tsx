/**
 * @module EmptyState
 * @description Reusable empty state component for lists and views.
 * Shows icon, message, and optional action button.
 * 
 * @implements FEAT0636 - Consistent empty state pattern
 * @implements FEAT0637 - Contextual empty state messaging
 * 
 * @enforces BR0623 - Empty states guide user to action
 * 
 * @see {@link docs/features.md} FEAT0636
 */
'use client';

import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { LucideIcon } from 'lucide-react';
import Link from 'next/link';

interface EmptyStateProps {
  icon?: LucideIcon;
  title: string;
  description?: string;
  action?: {
    label: string;
    href?: string;
    onClick?: () => void;
  };
  className?: string;
  children?: React.ReactNode;
}

export function EmptyState({
  icon: Icon,
  title,
  description,
  action,
  className,
  children,
}: EmptyStateProps) {
  return (
    <div
      className={cn(
        'flex flex-col items-center justify-center py-12 px-4 text-center',
        className
      )}
    >
      {Icon && (
        <div className="mx-auto mb-4 flex h-16 w-16 items-center justify-center rounded-full bg-muted">
          <Icon className="h-8 w-8 text-muted-foreground" />
        </div>
      )}
      <h3 className="text-lg font-semibold">{title}</h3>
      {description && (
        <p className="mt-2 text-sm text-muted-foreground max-w-sm">
          {description}
        </p>
      )}
      {action && (
        <div className="mt-6">
          {action.href ? (
            <Button asChild>
              <Link href={action.href}>{action.label}</Link>
            </Button>
          ) : action.onClick ? (
            <Button onClick={action.onClick}>{action.label}</Button>
          ) : null}
        </div>
      )}
      {children && <div className="mt-6">{children}</div>}
    </div>
  );
}

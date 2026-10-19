/**
 * @fileoverview Quick action navigation cards for common tasks
 *
 * @implements FEAT1010 - Quick action shortcuts
 * @implements FEAT1011 - Dashboard navigation widgets
 *
 * @see UC1103 - User navigates to documents/query/graph
 * @see UC1104 - User accesses primary workflows quickly
 *
 * @enforces BR1010 - Internationalized action labels
 * @enforces BR1011 - Accessible keyboard navigation
 */
'use client';

import { cn } from '@/lib/utils';
import { ArrowRight, FileText, MessageSquare, Network } from 'lucide-react';
import Link from 'next/link';
import { useTranslation } from 'react-i18next';

const actions = [
  {
    id: 'upload',
    href: '/documents',
    icon: FileText,
    labelKey: 'dashboard.quickActions.upload',
    descriptionKey: 'dashboard.quickActions.uploadDesc',
  },
  {
    id: 'query',
    href: '/query',
    icon: MessageSquare,
    labelKey: 'dashboard.quickActions.query',
    descriptionKey: 'dashboard.quickActions.queryDesc',
  },
  {
    id: 'graph',
    href: '/graph',
    icon: Network,
    labelKey: 'dashboard.quickActions.graph',
    descriptionKey: 'dashboard.quickActions.graphDesc',
  },
];

interface QuickActionsProps {
  /** Empty workspace: Upload is the one obvious next step. */
  emphasizeUpload?: boolean;
  /** `stack` = one column (side rail); `row` = three across. */
  layout?: 'row' | 'stack';
}

/**
 * Three compact, left-aligned shortcut tiles. No wrapper card or heading: the
 * actions are self-explanatory and a nested card made them read as placeholders.
 */
export function QuickActions({ emphasizeUpload = false, layout = 'row' }: QuickActionsProps) {
  const { t } = useTranslation();

  return (
    <div
      className={cn('grid min-w-0 gap-2 [&>*]:min-w-0', layout === 'row' && 'sm:grid-cols-3')}
      data-testid="dashboard-quick-actions"
    >
      {actions.map((action) => {
        const Icon = action.icon;
        const primary = emphasizeUpload && action.id === 'upload';
        return (
          <Link
            key={action.id}
            href={action.href}
            data-testid={`dashboard-action-${action.id}`}
            data-emphasis={primary ? 'primary' : undefined}
            className={cn(
              'group flex items-center gap-3 rounded-lg border px-3 py-2.5',
              'transition-colors duration-150',
              'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
              primary
                ? 'border-primary bg-primary text-primary-foreground hover:bg-primary/90'
                : 'bg-card hover:bg-muted/50',
            )}
          >
            <div
              className={cn(
                'flex h-9 w-9 shrink-0 items-center justify-center rounded-md',
                primary ? 'bg-primary-foreground/15' : 'bg-muted',
              )}
            >
              <Icon className="h-4 w-4" aria-hidden="true" />
            </div>
            <div className="min-w-0 flex-1">
              <p className="truncate text-sm font-medium">{t(action.labelKey)}</p>
              <p
                className={cn(
                  cn('truncate text-xs', layout === 'row' && 'hidden sm:block'),
                  primary ? 'text-primary-foreground/80' : 'text-muted-foreground',
                )}
              >
                {t(action.descriptionKey)}
              </p>
            </div>
            <ArrowRight
              className="h-4 w-4 shrink-0 opacity-0 transition-opacity group-hover:opacity-60 group-focus-visible:opacity-60 motion-reduce:transition-none"
              aria-hidden="true"
            />
          </Link>
        );
      })}
    </div>
  );
}

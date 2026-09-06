'use client';

import { Button } from '@/components/ui/button';
import { formatContextLabels } from '@/lib/layout/format-context-labels';
import { cn } from '@/lib/utils';
import { ChevronDown, FolderKanban, Loader2 } from 'lucide-react';
import { forwardRef } from 'react';
import { useTranslation } from 'react-i18next';

export interface ContextTriggerChipProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  tenantName?: string | null;
  workspaceName?: string | null;
  /** SPEC-140: chip is current-only; count documents the inventory. */
  workspaceCount?: number;
  isLoading?: boolean;
  open?: boolean;
  className?: string;
}

/**
 * SPEC-101 LAW-101-11 — One-line `Tenant — Workspace` context chip.
 * Full names live in title / aria / data-full-name; chrome may smartTruncate.
 */
export const ContextTriggerChip = forwardRef<HTMLButtonElement, ContextTriggerChipProps>(
  function ContextTriggerChip(
    { tenantName, workspaceName, workspaceCount, isLoading, open, className, ...rest },
    ref,
  ) {
    const { t } = useTranslation();
    const labels = formatContextLabels(
      { tenantName, workspaceName },
      {
        selectTenant: t('context.selectTenant', 'Select tenant'),
        selectWorkspace: t('context.selectWorkspace', 'Select workspace'),
        tenantLabel: t('context.tenantLabel', 'Tenant'),
        workspaceLabel: t('context.workspaceLabel', 'Workspace'),
        maxLen: 18,
      },
    );

    return (
      <Button
        ref={ref}
        type="button"
        data-testid="workspace-selector"
        data-workspace-count={
          typeof workspaceCount === 'number' ? String(workspaceCount) : undefined
        }
        variant="ghost"
        size="sm"
        role="combobox"
        aria-expanded={open}
        aria-label={labels.ariaLabel}
        title={labels.title}
        className={cn(
          'h-8 gap-1.5 px-2.5 font-medium text-sm',
          'bg-muted/50 hover:bg-muted border border-border/50',
          'min-w-0 max-w-[min(28rem,42vw)]',
          'transition-all duration-150',
          className,
        )}
        {...rest}
      >
        {isLoading ? (
          <Loader2 className="h-3.5 w-3.5 shrink-0 animate-spin" />
        ) : (
          <FolderKanban
            className="h-3.5 w-3.5 shrink-0 text-muted-foreground"
            aria-hidden="true"
          />
        )}
        <span
          className="flex min-w-0 flex-1 items-baseline gap-1 text-left text-xs font-medium"
          data-testid="context-line"
        >
          <span
            className={cn('min-w-0 truncate', !labels.hasTenant && 'text-muted-foreground')}
            data-testid="context-tenant-label"
            data-full-name={labels.tenantDisplay}
          >
            {labels.tenantShort}
          </span>
          {labels.hasTenant ? (
            <span className="shrink-0 text-muted-foreground" aria-hidden="true">
              —
            </span>
          ) : null}
          {labels.hasTenant ? (
            <span
              className={cn(
                'min-w-0 truncate',
                !labels.hasWorkspace && 'text-amber-700 dark:text-amber-400',
              )}
              data-testid="context-workspace-label"
              data-full-name={labels.workspaceDisplay}
            >
              {labels.workspaceShort}
            </span>
          ) : null}
        </span>
        <ChevronDown className="h-3 w-3 shrink-0 text-muted-foreground" aria-hidden="true" />
      </Button>
    );
  },
);

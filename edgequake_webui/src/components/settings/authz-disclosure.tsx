'use client';

/**
 * Shared Show/Hide disclosure for SPEC-146 Settings (Roles create, Policies Cedar).
 * One outline-button pattern — do not invent a third Advanced style.
 */

import type { ReactNode } from 'react';
import { Button } from '@/components/ui/button';
import { useTranslation } from 'react-i18next';

export interface AuthzDisclosureProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Shown when collapsed (e.g. "Create custom role"). */
  showLabel: string;
  /** Optional hint left of the toggle. */
  hint?: string;
  testId: string;
  children: ReactNode;
  /** Extra class on the expanded panel. */
  panelClassName?: string;
  panelTestId?: string;
}

export function AuthzDisclosure({
  open,
  onOpenChange,
  showLabel,
  hint,
  testId,
  children,
  panelClassName = 'space-y-2 rounded-md border p-3',
  panelTestId,
}: AuthzDisclosureProps) {
  const { t } = useTranslation();
  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between gap-2">
        {hint ? (
          <p className="text-xs text-muted-foreground min-w-0">{hint}</p>
        ) : (
          <span />
        )}
        <Button
          type="button"
          size="sm"
          variant="outline"
          data-testid={testId}
          onClick={() => onOpenChange(!open)}
        >
          {open ? t('common.hide', 'Hide') : showLabel}
        </Button>
      </div>
      {open ? (
        <div className={panelClassName} data-testid={panelTestId}>
          {children}
        </div>
      ) : null}
    </div>
  );
}

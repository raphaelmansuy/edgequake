'use client';

/**
 * SPEC-146 quarantine chrome SSOT — table chip path + detail Security panel.
 */

import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { ShieldAlert } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export function QuarantineBanner({
  onRetry,
  retryTestId = 'spec146-retry-labels-detail',
  canSetLabels = true,
}: {
  onRetry?: () => void;
  retryTestId?: string;
  /** Hide Retry when caller lacks document:set_labels. */
  canSetLabels?: boolean;
}) {
  const { t } = useTranslation();
  return (
    <Alert
      data-testid="spec146-quarantine-banner"
      role="status"
      aria-live="polite"
    >
      <ShieldAlert className="h-4 w-4" />
      <AlertTitle>
        {t('security.quarantined', 'Labeling failed / Quarantined')}
      </AlertTitle>
      <AlertDescription className="space-y-2">
        <p>
          {t(
            'security.quarantineBanner',
            'Security labels could not be applied. This document isn’t searchable until labeling succeeds.',
          )}
        </p>
        {onRetry && canSetLabels ? (
          <Button
            type="button"
            size="sm"
            variant="outline"
            className="mt-1"
            data-testid={retryTestId}
            onClick={onRetry}
          >
            {t('security.retryLabels', 'Retry labels')}
          </Button>
        ) : null}
        {onRetry && !canSetLabels ? (
          <p className="text-xs text-muted-foreground">
            {t(
              'security.noSetLabels',
              'You don’t have permission to change security labels.',
            )}
          </p>
        ) : null}
      </AlertDescription>
    </Alert>
  );
}

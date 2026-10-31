'use client';

/**
 * SPEC-180 08/11 — the image embedding probe, shown on the model step.
 *
 * The spec is explicit: a failing create probe is "inline error on the model step … do not
 * advance to review". A footer sentence is not enough — the user needs to see *which* pair
 * fails and *what* to change while the picker is still in front of them.
 */

import type { WizardEligibility } from '@/hooks/use-wizard-eligibility';
import { AlertTriangle, CheckCircle2, Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export interface ImageEmbeddingProbeNoticeProps {
  /** Only an image-embedding draft has a probe to report. */
  enabled: boolean;
  probe: WizardEligibility['imageProbe'];
  /** The pair being judged, so the sentence can name it. */
  pair?: { provider?: string; model?: string };
}

export function ImageEmbeddingProbeNotice({ enabled, probe, pair }: ImageEmbeddingProbeNoticeProps) {
  const { t } = useTranslation();
  if (!enabled || probe.status === 'idle') return null;
  const pairLabel =
    pair?.provider && pair?.model ? `${pair.provider}/${pair.model}` : null;

  if (probe.status === 'checking') {
    return (
      <p
        className="flex items-start gap-1.5 text-xs text-muted-foreground"
        data-testid="wizard-image-probe-checking"
        role="status"
      >
        <Loader2 aria-hidden className="mt-px h-3.5 w-3.5 shrink-0 animate-spin" />
        {t(
          'ragProfile.probe.checking',
          'Checking that this deployment can embed images with {{model}}…',
          { model: pairLabel ?? t('ragProfile.probe.thisModel', 'this model') },
        )}
      </p>
    );
  }

  if (probe.status === 'failed') {
    return (
      <div
        className="flex items-start gap-2 rounded-lg border border-amber-300/70 bg-amber-50/70 p-2.5 text-xs text-amber-900 dark:border-amber-900 dark:bg-amber-950/30 dark:text-amber-200"
        data-testid="wizard-image-probe-failed"
        role="alert"
      >
        <AlertTriangle aria-hidden className="mt-px h-3.5 w-3.5 shrink-0" />
        <div className="min-w-0 space-y-1">
          <p className="font-medium">
            {t(
              'ragProfile.probe.failedTitle',
              'This deployment cannot embed images with this model',
            )}
          </p>
          <p className="leading-relaxed">
            {probe.message ??
              t(
                'ragProfile.probe.failedBody',
                'Choose another image embedding model, or ask a deployment admin to fix its credentials.',
              )}
          </p>
          {pairLabel ? (
            <p className="text-amber-800/80 dark:text-amber-300/80">
              {t('ragProfile.probe.pair', 'Model: {{model}}', { model: pairLabel })}
            </p>
          ) : null}
        </div>
      </div>
    );
  }

  return (
    <p
      className="flex items-center gap-1.5 text-xs text-emerald-700 dark:text-emerald-400"
      data-testid="wizard-image-probe-ok"
      role="status"
    >
      <CheckCircle2 aria-hidden className="h-3.5 w-3.5 shrink-0" />
      {t(
        'ragProfile.probe.ok',
        'This deployment can embed images with {{model}}.',
        { model: pairLabel ?? t('ragProfile.probe.thisModel', 'this model') },
      )}
    </p>
  );
}

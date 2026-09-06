'use client';

/**
 * SPEC-146 M5: banner when an active break-glass session exists (LAW-146-25 / G-146-50).
 * Copy must not leak deny reasons, hidden counts, or Restricted titles.
 * Normative short deck (06-ux): primary line + expiry as secondary detail.
 */

import { listBreakGlass } from '@/lib/api/edgequake/authz';
import { useDocAbacEnabled } from '@/hooks/use-doc-abac';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useQuery } from '@tanstack/react-query';
import { ShieldAlert } from 'lucide-react';
import { useTranslation } from 'react-i18next';

function isSessionActive(expiresAt: string, revokedAt?: string | null): boolean {
  if (revokedAt) return false;
  const exp = Date.parse(expiresAt);
  if (Number.isNaN(exp)) return false;
  return exp > Date.now();
}

function formatExpiry(iso: string): string {
  const exp = Date.parse(iso);
  if (Number.isNaN(exp)) return iso;
  return new Date(exp).toLocaleString(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  });
}

export function BreakGlassBanner() {
  const { t } = useTranslation();
  const { docAbacEnabled, isLoading } = useDocAbacEnabled();
  const workspaceId = useTenantStore((s) => s.selectedWorkspaceId);

  const { data } = useQuery({
    queryKey: ['authz', 'break-glass', workspaceId],
    queryFn: () => listBreakGlass(workspaceId!),
    enabled: Boolean(docAbacEnabled && workspaceId),
    refetchInterval: 60_000,
    staleTime: 15_000,
  });

  if (isLoading || !docAbacEnabled || !workspaceId) return null;

  const active = (data?.sessions ?? []).filter((s) =>
    isSessionActive(s.expires_at, s.revoked_at),
  );
  if (active.length === 0) return null;

  const soonest = active.reduce((a, b) =>
    Date.parse(a.expires_at) <= Date.parse(b.expires_at) ? a : b,
  );

  return (
    <div
      role="alert"
      aria-live="assertive"
      data-testid="spec146-break-glass-banner"
      className="flex items-start gap-2 border-b border-amber-200 bg-amber-50 px-4 py-2 text-sm text-amber-900 dark:border-amber-900/50 dark:bg-amber-950/80 dark:text-amber-100"
    >
      <ShieldAlert className="h-4 w-4 shrink-0 mt-0.5" aria-hidden="true" />
      <div className="flex-1 min-w-0">
        <p>
          {t(
            'security.breakGlass.banner',
            'Break-glass session active. Actions are audited.',
          )}
        </p>
        <p className="text-xs text-amber-800/80 dark:text-amber-200/80 mt-0.5">
          {t('security.breakGlass.until', 'Until {{expires}}.', {
            expires: formatExpiry(soonest.expires_at),
          })}
        </p>
      </div>
    </div>
  );
}

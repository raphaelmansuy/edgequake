'use client';

/**
 * @module identity-providers-card
 * @description SPEC-158 W5 — platform-admin, read-only overview of SSO providers.
 * Mutations go through `PUT /admin/identity-providers/{slug}` (secrets via env references only).
 */

import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { useSsoProviders } from '@/hooks/use-sso-providers';
import { listIdentityProviders, type StoredIdentityProvider } from '@/lib/api/edgequake/sso';
import { useAuthStore } from '@/stores/use-auth-store';
import { useQuery } from '@tanstack/react-query';
import { KeyRound } from 'lucide-react';
import { useTranslation } from 'react-i18next';

function truncateIssuer(issuer: string): string {
  return issuer.length > 48 ? `${issuer.slice(0, 45)}…` : issuer;
}

export function IdentityProvidersCard() {
  const { t } = useTranslation();
  const user = useAuthStore((s) => s.user);
  const hydrated = useAuthStore((s) => s._hasHydrated);
  const isAdmin = user?.role === 'admin' || user?.roles?.includes('admin') || false;

  const active = useSsoProviders();
  const stored = useQuery<StoredIdentityProvider[]>({
    queryKey: ['admin', 'identity-providers'],
    queryFn: listIdentityProviders,
    enabled: hydrated && isAdmin,
    retry: 0,
  });

  if (!hydrated || !isAdmin) return null;

  const storedBySlug = new Map((stored.data ?? []).map((p) => [p.slug, p]));
  const rows = Array.isArray(active.data) ? active.data : [];

  return (
    <Card data-testid="identity-providers-card">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <KeyRound className="h-5 w-5" aria-hidden />
          {t('auth.sso.admin.title', 'Identity providers')}
        </CardTitle>
        <CardDescription>
          {t(
            'auth.sso.admin.description',
            'Single sign-on providers available at login. Environment-configured providers are read-only here.',
          )}
        </CardDescription>
      </CardHeader>
      <CardContent>
        {active.isLoading || stored.isLoading ? (
          <Skeleton className="h-16 w-full" />
        ) : rows.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            {t('auth.sso.admin.empty', 'No SSO provider is configured (password login only).')}
          </p>
        ) : (
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-muted-foreground">
                <th className="py-1 pr-3 font-medium">{t('auth.sso.admin.provider', 'Provider')}</th>
                <th className="py-1 pr-3 font-medium">{t('auth.sso.admin.issuer', 'Issuer')}</th>
                <th className="py-1 font-medium">{t('auth.sso.admin.policy', 'Policy')}</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((p) => {
                const detail = storedBySlug.get(p.slug);
                return (
                  <tr key={p.slug} className="border-t" data-testid={`idp-row-${p.slug}`}>
                    <td className="py-2 pr-3">
                      <div className="font-medium">{p.display_name}</div>
                      <div className="text-xs text-muted-foreground">
                        {p.slug} · {p.kind}
                      </div>
                    </td>
                    <td className="py-2 pr-3 font-mono text-xs">
                      {detail ? truncateIssuer(detail.issuer) : t('auth.sso.admin.env', 'environment')}
                    </td>
                    <td className="space-x-1 py-2">
                      {detail ? (
                        <>
                          <Badge variant={detail.trust_email ? 'default' : 'secondary'}>
                            {detail.trust_email
                              ? t('auth.sso.admin.trustEmail', 'trusts email')
                              : t('auth.sso.admin.noTrustEmail', 'email not trusted')}
                          </Badge>
                          <Badge variant="outline">max {detail.max_role}</Badge>
                          {!detail.jit_enabled && <Badge variant="outline">JIT off</Badge>}
                        </>
                      ) : (
                        <Badge variant="secondary">{t('auth.sso.admin.env', 'environment')}</Badge>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </CardContent>
    </Card>
  );
}

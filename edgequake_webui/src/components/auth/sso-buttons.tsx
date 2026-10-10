'use client';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { useSsoProviders } from '@/hooks/use-sso-providers';
import { buildSsoLoginUrl, type SsoProvider } from '@/lib/api/edgequake/sso';
import { KeyRound, Loader2 } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

interface SsoButtonsProps {
  /** Same-origin path to land on after login. */
  redirect?: string | null;
  /** Prefilled organization alias (e.g. from an `org_ambiguous` picker). */
  defaultOrg?: string;
}

/** Only Keycloak Organizations carry an org hint; other IdPs ignore it. */
function acceptsOrgHint(provider: SsoProvider): boolean {
  return provider.kind === 'keycloak';
}

/**
 * SPEC-158 — "Continue with …" buttons (full-page navigation to the API, never a fetch).
 * Renders nothing when no provider is configured, so password-only installs are unchanged.
 */
export function SsoButtons({ redirect, defaultOrg = '' }: SsoButtonsProps) {
  const { t } = useTranslation();
  const { data: providers, isLoading } = useSsoProviders();
  const [org, setOrg] = useState(defaultOrg);
  const [pending, setPending] = useState<string | null>(null);

  if (isLoading) {
    return (
      <div
        className="h-10 w-full rounded-md bg-muted"
        aria-busy="true"
        data-testid="sso-buttons-skeleton"
      />
    );
  }

  if (!providers || providers.length === 0) return null;

  const showOrg = providers.some(acceptsOrgHint);
  const start = (provider: SsoProvider) => {
    setPending(provider.slug);
    window.location.assign(
      buildSsoLoginUrl({
        provider: provider.slug,
        org: acceptsOrgHint(provider) ? org : undefined,
        redirect,
      }),
    );
  };

  return (
    <div className="space-y-3" data-testid="sso-buttons">
      {showOrg && (
        <div className="space-y-1">
          <label htmlFor="sso-org" className="text-sm font-medium">
            {t('auth.sso.orgLabel', 'Organization (optional)')}
          </label>
          <Input
            id="sso-org"
            name="org"
            autoComplete="organization"
            placeholder={t('auth.sso.orgPlaceholder', 'your-company')}
            value={org}
            onChange={(e) => setOrg(e.target.value)}
            disabled={pending !== null}
            aria-describedby="sso-org-hint"
          />
          <p id="sso-org-hint" className="text-xs text-muted-foreground">
            {t('auth.sso.orgHint', 'Leave empty if you belong to a single organization.')}
          </p>
        </div>
      )}
      {providers.map((provider) => (
        <Button
          key={provider.slug}
          type="button"
          variant="outline"
          className="w-full"
          disabled={pending !== null}
          data-testid={`sso-provider-${provider.slug}`}
          onClick={() => start(provider)}
        >
          {pending === provider.slug ? (
            <Loader2 className="mr-2 h-4 w-4 animate-spin" aria-hidden />
          ) : (
            <KeyRound className="mr-2 h-4 w-4" aria-hidden />
          )}
          {t('auth.sso.continueWith', {
            name: provider.display_name,
            defaultValue: 'Continue with {{name}}',
          })}
        </Button>
      ))}
      <div className="relative my-2">
        <div className="absolute inset-0 flex items-center">
          <span className="w-full border-t" />
        </div>
        <div className="relative flex justify-center text-xs">
          <span className="bg-background px-2 text-muted-foreground">
            {t('auth.sso.orLocal', 'or sign in with a password')}
          </span>
        </div>
      </div>
    </div>
  );
}

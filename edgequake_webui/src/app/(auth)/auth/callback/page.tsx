'use client';

import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { buildSsoLoginUrl } from '@/lib/api/edgequake/sso';
import { completeSsoLogin } from '@/lib/auth/complete-sso-login';
import { ambiguousOrgAliases, ssoErrorKey } from '@/lib/auth/sso-errors';
import { AlertCircle, Loader2 } from 'lucide-react';
import { useRouter, useSearchParams } from 'next/navigation';
import { Suspense, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

/**
 * SPEC-158 — SSO landing page. The API redirects here with `?code=` (success) or `?error=<code>`.
 * No token ever appears in this URL; the code is single-use and redeemed exactly once.
 */
function SsoCallbackInner() {
  const { t } = useTranslation();
  const router = useRouter();
  const params = useSearchParams();
  const code = params.get('code');
  const errorCode = params.get('error');
  const [failure, setFailure] = useState<string | null>(errorCode);
  const redeemed = useRef(false);

  useEffect(() => {
    if (!code || redeemed.current) return;
    redeemed.current = true; // React strict-mode double effect must not burn the code twice
    completeSsoLogin(code)
      .then(({ redirectTo }) => router.replace(redirectTo))
      .catch(() => setFailure('invalid_handoff'));
  }, [code, router]);

  const error = failure ?? (!code ? 'invalid_handoff' : null);
  const aliases = ambiguousOrgAliases(error);

  if (!error) {
    return (
      <Card className="w-full max-w-md min-h-48" data-testid="sso-completing">
        <CardContent
          role="status"
          className="flex min-h-48 flex-col items-center justify-center gap-3 text-muted-foreground"
        >
          <Loader2 className="h-6 w-6 animate-spin" aria-hidden />
          <p className="text-sm">{t('auth.sso.completing', 'Completing sign-in…')}</p>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className="w-full max-w-md" data-testid="sso-error">
      <CardHeader>
        {aliases.length > 0 ? (
          <CardTitle>{t('auth.sso.pickOrg', 'Choose an organization')}</CardTitle>
        ) : (
          <CardTitle className="flex items-center gap-2 text-destructive">
            <AlertCircle className="h-5 w-5" aria-hidden />
            {t('auth.sso.errors.title', 'Sign-in failed')}
          </CardTitle>
        )}
      </CardHeader>
      <CardContent className="space-y-4">
        <p role="alert" data-testid="sso-error-message" className="text-sm">
          {t(ssoErrorKey(error))}
        </p>
        {aliases.length > 0 && (
          <div className="space-y-2" data-testid="sso-org-picker">
            {aliases.map((alias) => (
              <Button
                key={alias}
                variant="outline"
                className="w-full"
                onClick={() => window.location.assign(buildSsoLoginUrl({ org: alias }))}
              >
                {alias}
              </Button>
            ))}
          </div>
        )}
        <Button variant="secondary" className="w-full" onClick={() => router.replace('/login')}>
          {t('auth.sso.backToLogin', 'Back to sign in')}
        </Button>
      </CardContent>
    </Card>
  );
}

export default function SsoCallbackPage() {
  return (
    <div className="flex h-full min-h-0 items-center justify-center bg-background p-4">
      <Suspense fallback={<Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />}>
        <SsoCallbackInner />
      </Suspense>
    </div>
  );
}

'use client';

import { FirstRunWizard } from '@/components/onboarding/first-run-wizard';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { useSetupStatus } from '@/hooks/use-setup-status';
import { SsoButtons } from '@/components/auth/sso-buttons';
import { login, safeRedirectPath } from '@/lib/api/edgequake';
import { getRuntimeConfig } from '@/lib/runtime-config';
import { useAuthStore } from '@/stores/use-auth-store';
import { AlertCircle, Eye, EyeOff, Loader2, Network } from 'lucide-react';
import { useRouter, useSearchParams } from 'next/navigation';
import { Suspense, useState } from 'react';

function LoginPageInner() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const authLogin = useAuthStore((s) => s.login);
  const { data: setupStatus, isLoading: setupLoading } = useSetupStatus();

  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { authEnabled, disableDemoLogin } = getRuntimeConfig();
  const showDemoLogin = !disableDemoLogin && !authEnabled;
  // SPEC-155: default landing is dashboard, not /graph (F-155-P12)
  const postLoginPath =
    safeRedirectPath(searchParams.get('redirect') ?? searchParams.get('next')) ?? '/';

  if (setupLoading) {
    return (
      <div
        className="flex h-full min-h-0 items-center justify-center bg-background p-4"
        aria-busy="true"
        data-testid="login-setup-skeleton"
      >
        <Card className="w-full max-w-md">
          <CardHeader>
            <div className="h-6 w-40 rounded bg-muted" />
            <div className="h-4 w-64 rounded bg-muted" />
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="h-10 w-full rounded bg-muted" />
            <div className="h-10 w-full rounded bg-muted" />
          </CardContent>
        </Card>
      </div>
    );
  }

  if (setupStatus?.needs_setup && setupStatus.auth_enabled) {
    return (
      <div className="flex h-full min-h-0 items-center justify-center overflow-y-auto bg-background p-4">
        <FirstRunWizard surface="login" />
      </div>
    );
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setIsLoading(true);

    try {
      const response = await login({ username, password });
      authLogin(response);
      router.push(postLoginPath);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Login failed';
      // Single error surface — no toast + alert (SPEC-155 F-155-P12)
      setError(message);
    } finally {
      setIsLoading(false);
    }
  };

  const handleSkipLogin = () => {
    router.push(postLoginPath);
  };

  return (
    <div className="flex h-full min-h-0 items-center justify-center overflow-y-auto bg-background p-4">
      <Card className="w-full max-w-md">
        <CardHeader className="text-center">
          <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-primary/10">
            <Network className="h-6 w-6 text-primary" />
          </div>
          <CardTitle className="text-2xl font-bold">EdgeQuake</CardTitle>
          <CardDescription>
            Sign in to access the Knowledge Graph RAG Platform
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <SsoButtons redirect={postLoginPath} />
          <form
            onSubmit={handleSubmit}
            className="space-y-4"
            aria-describedby={error ? 'login-error' : undefined}
          >
            <div className="space-y-2">
              <label htmlFor="username" className="text-sm font-medium">
                Username
              </label>
              <Input
                id="username"
                name="username"
                type="text"
                autoComplete="username"
                placeholder="Enter your username"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                disabled={isLoading}
                required
                aria-required="true"
                aria-invalid={error ? 'true' : undefined}
              />
            </div>
            <div className="space-y-2">
              <label htmlFor="password" className="text-sm font-medium">
                Password
              </label>
              <div className="relative">
                <Input
                  id="password"
                  name="password"
                  type={showPassword ? 'text' : 'password'}
                  autoComplete="current-password"
                  placeholder="Enter your password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  disabled={isLoading}
                  required
                  aria-required="true"
                  aria-invalid={error ? 'true' : undefined}
                  className="pr-10"
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  className="absolute right-1 top-1/2 h-8 w-8 -translate-y-1/2"
                  onClick={() => setShowPassword((v) => !v)}
                  aria-label={showPassword ? 'Hide password' : 'Show password'}
                >
                  {showPassword ? (
                    <EyeOff className="h-4 w-4" aria-hidden />
                  ) : (
                    <Eye className="h-4 w-4" aria-hidden />
                  )}
                </Button>
              </div>
            </div>

            {error && (
              <div
                id="login-error"
                role="alert"
                aria-live="assertive"
                className="flex items-start gap-2 rounded-md bg-destructive/10 p-3 text-sm text-destructive"
              >
                <AlertCircle className="mt-0.5 h-4 w-4 shrink-0" aria-hidden="true" />
                <span>{error}</span>
              </div>
            )}

            <Button type="submit" className="w-full" disabled={isLoading}>
              {isLoading ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Signing in...
                </>
              ) : (
                'Sign In'
              )}
            </Button>

            {showDemoLogin && (
              <>
                <div className="relative my-4">
                  <div className="absolute inset-0 flex items-center">
                    <span className="w-full border-t" />
                  </div>
                  <div className="relative flex justify-center text-xs uppercase">
                    <span className="bg-background px-2 text-muted-foreground">Or</span>
                  </div>
                </div>

                <Button
                  type="button"
                  variant="outline"
                  className="w-full"
                  onClick={handleSkipLogin}
                >
                  Continue without login (Demo)
                </Button>
              </>
            )}
          </form>
        </CardContent>
      </Card>
    </div>
  );
}

export default function LoginPage() {
  return (
    <Suspense
      fallback={
        <div className="flex h-full min-h-0 items-center justify-center bg-background p-4">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
        </div>
      }
    >
      <LoginPageInner />
    </Suspense>
  );
}

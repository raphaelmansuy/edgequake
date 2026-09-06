'use client';

import { FirstRunWizard } from '@/components/onboarding/first-run-wizard';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { useSetupStatus } from '@/hooks/use-setup-status';
import { login } from '@/lib/api/edgequake';
import { getRuntimeConfig } from '@/lib/runtime-config';
import { useAuthStore } from '@/stores/use-auth-store';
import { AlertCircle, Loader2, Network } from 'lucide-react';
import { useRouter } from 'next/navigation';
import { useState } from 'react';
import { toast } from 'sonner';

/** Well-known local-dev credentials pinned by `make dev` (never for production). */
export const DEV_LOGIN_USERNAME = 'admin';
export const DEV_LOGIN_PASSWORD = 'EdgeQuake1';

export default function LoginPage() {
  const router = useRouter();
  const authLogin = useAuthStore((s) => s.login);
  const { data: setupStatus, isLoading: setupLoading } = useSetupStatus();

  const { authEnabled, disableDemoLogin, showDevLoginHint } = getRuntimeConfig();
  const showDemoLogin = !disableDemoLogin && !authEnabled;

  const [username, setUsername] = useState(
    showDevLoginHint ? DEV_LOGIN_USERNAME : ''
  );
  const [password, setPassword] = useState(
    showDevLoginHint ? DEV_LOGIN_PASSWORD : ''
  );
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // SPEC-101: empty auth-on install → first-run wizard instead of login form
  if (!setupLoading && setupStatus?.needs_setup && setupStatus.auth_enabled) {
    return (
      <div className="flex h-full min-h-0 items-center justify-center overflow-y-auto bg-gradient-to-br from-background to-muted/50 p-4">
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
      toast.success('Successfully logged in');
      router.push('/graph');
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Login failed';
      setError(message);
      toast.error(message);
    } finally {
      setIsLoading(false);
    }
  };

  const handleSkipLogin = () => {
    // For development/demo mode without auth (make dev-open escape hatch)
    router.push('/graph');
  };

  return (
    <div className="flex h-full min-h-0 items-center justify-center overflow-y-auto bg-gradient-to-br from-background to-muted/50 p-4">
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
        <CardContent>
          {showDevLoginHint && (
            <div
              data-testid="dev-login-hint"
              className="mb-4 rounded-md border border-dashed border-primary/40 bg-primary/5 p-3 text-sm"
            >
              <p className="font-medium text-foreground">Local development login</p>
              <p className="mt-1 text-muted-foreground">
                Username:{' '}
                <code className="rounded bg-muted px-1 py-0.5 font-mono text-foreground">
                  {DEV_LOGIN_USERNAME}
                </code>
                {' · '}
                Password:{' '}
                <code className="rounded bg-muted px-1 py-0.5 font-mono text-foreground">
                  {DEV_LOGIN_PASSWORD}
                </code>
              </p>
              <p className="mt-1 text-xs text-muted-foreground">
                Pinned by <code className="font-mono">make dev</code> — not for production.
              </p>
            </div>
          )}

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
                type="text"
                placeholder="Enter your username"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                disabled={isLoading}
                required
                aria-required="true"
                aria-invalid={error ? 'true' : undefined}
                autoComplete="username"
              />
            </div>
            <div className="space-y-2">
              <label htmlFor="password" className="text-sm font-medium">
                Password
              </label>
              <Input
                id="password"
                type="password"
                placeholder="Enter your password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                disabled={isLoading}
                required
                aria-required="true"
                aria-invalid={error ? 'true' : undefined}
                autoComplete="current-password"
              />
            </div>

            {error && (
              <div
                id="login-error"
                role="alert"
                aria-live="assertive"
                className="rounded-md bg-destructive/10 p-3 text-sm text-destructive flex items-start gap-2"
              >
                <AlertCircle className="h-4 w-4 shrink-0 mt-0.5" aria-hidden="true" />
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
                    <span className="bg-background px-2 text-muted-foreground">
                      Or
                    </span>
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

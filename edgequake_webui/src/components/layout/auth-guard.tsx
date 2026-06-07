'use client';

import { getRuntimeConfig } from '@/lib/runtime-config';
import { useAuthStore, useAuthStoreHydrated } from '@/stores/use-auth-store';
import { Loader2 } from 'lucide-react';
import { usePathname, useRouter } from 'next/navigation';
import { useEffect } from 'react';

interface AuthGuardProps {
  children: React.ReactNode;
}

export function AuthGuard({ children }: AuthGuardProps) {
  const router = useRouter();
  const pathname = usePathname();
  const isAuthenticated = useAuthStore((state) => state.isAuthenticated);
  const accessToken = useAuthStore((state) => state.accessToken);
  const isTokenExpired = useAuthStore((state) => state.isTokenExpired);
  const logout = useAuthStore((state) => state.logout);
  const hasHydrated = useAuthStoreHydrated();
  const { authEnabled, disableDemoLogin } = getRuntimeConfig();

  const requiresAuth = authEnabled || disableDemoLogin;
  // A session is valid only if authenticated, has a token, AND the token is not expired
  const hasSession = isAuthenticated && !!accessToken && !isTokenExpired();

  // Listen for API-level auth failures (e.g. 401 after failed token refresh)
  useEffect(() => {
    const handleAuthFailure = () => {
      logout();
      router.replace('/login');
    };
    window.addEventListener('auth:logout-required', handleAuthFailure);
    return () => window.removeEventListener('auth:logout-required', handleAuthFailure);
  }, [logout, router]);

  useEffect(() => {
    if (hasHydrated && requiresAuth && !hasSession && pathname !== '/login') {
      router.replace('/login');
    }
  }, [hasHydrated, hasSession, pathname, requiresAuth, router]);

  if (requiresAuth && !hasHydrated) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="text-center">
          <Loader2 className="mx-auto mb-3 h-8 w-8 animate-spin text-muted-foreground" />
          <p className="text-sm text-muted-foreground">Checking session...</p>
        </div>
      </div>
    );
  }

  if (requiresAuth && !hasSession) {
    return null;
  }

  return <>{children}</>;
}

export default AuthGuard;

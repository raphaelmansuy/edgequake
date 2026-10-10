'use client';

import { restoreSessionFromRefreshCookie } from '@/lib/api/client';
import { setTokens } from '@/lib/api/client-context';
import {
  SOFT_EXPIRY_LEAD_MS,
  scheduleAccessTokenRefresh,
} from '@/lib/auth/session-keepalive';
import { getRuntimeConfig } from '@/lib/runtime-config';
import { useAuthStore, useAuthStoreHydrated } from '@/stores/use-auth-store';
import { Loader2 } from 'lucide-react';
import { usePathname, useRouter } from 'next/navigation';
import { useEffect, useRef, useState } from 'react';

interface AuthGuardProps {
  children: React.ReactNode;
}

declare global {
  interface Window {
    /** E2E-only: memory access token seed (SPEC-154 — not localStorage). */
    __eqE2ePendingToken?: string;
    /** E2E-only: force soft-expiry so guard redeems `eq_refresh` (boot path). */
    __eqE2eSoftExpireAccess?: () => void;
    /**
     * E2E-only: keep session valid but schedule keepalive soon
     * (`expiresAt = now + SOFT_EXPIRY_LEAD_MS + delayMs`).
     */
    __eqE2eScheduleKeepaliveSoon?: (delayMs?: number) => void;
  }
}

export function AuthGuard({ children }: AuthGuardProps) {
  const router = useRouter();
  const pathname = usePathname();
  const isAuthenticated = useAuthStore((state) => state.isAuthenticated);
  const accessToken = useAuthStore((state) => state.accessToken);
  const expiresAt = useAuthStore((state) => state.expiresAt);
  const isTokenExpired = useAuthStore((state) => state.isTokenExpired);
  const logout = useAuthStore((state) => state.logout);
  const hasHydrated = useAuthStoreHydrated();
  const { authEnabled, disableDemoLogin } = getRuntimeConfig();
  /** UI: hold "Checking session…" while cookie redeem is in flight. */
  const [cookieProbe, setCookieProbe] = useState<'idle' | 'probing' | 'done'>('idle');
  /** True while a restore is in flight (or succeeded until hasSession flips). */
  const probing = useRef(false);
  /** True after restore failed and we started navigating to /login. */
  const redirecting = useRef(false);
  /** Once a session painted, keep the tree mounted while the refresh cookie redeems. */
  const seenSession = useRef(false);

  // Playwright cannot write SPEC-154 memory tokens via localStorage. Specs
  // seed `window.__eqE2ePendingToken` before navigation; adopt it once.
  useEffect(() => {
    const pending = window.__eqE2ePendingToken;
    if (!pending) return;
    setTokens(pending);
    useAuthStore.setState({
      isAuthenticated: true,
      accessToken: pending,
      expiresAt: Date.now() + 3_600_000,
    });
    delete window.__eqE2ePendingToken;
  }, []);

  // E2E hooks (non-production only).
  useEffect(() => {
    if (process.env.NODE_ENV === 'production') return;
    window.__eqE2eSoftExpireAccess = () => {
      useAuthStore.setState({ expiresAt: Date.now() - 1 });
    };
    window.__eqE2eScheduleKeepaliveSoon = (delayMs = 150) => {
      const delay = Math.max(0, delayMs);
      useAuthStore.setState({
        expiresAt: Date.now() + SOFT_EXPIRY_LEAD_MS + delay,
      });
    };
    return () => {
      delete window.__eqE2eSoftExpireAccess;
      delete window.__eqE2eScheduleKeepaliveSoon;
    };
  }, []);

  const requiresAuth = authEnabled || disableDemoLogin;
  // Soft-expiry (5m buffer) means "refresh needed", not permanent logout.
  const hasSession = isAuthenticated && !!accessToken && !isTokenExpired();
  if (hasSession) seenSession.current = true;

  // Listen for API-level auth failures (e.g. 401 after failed token refresh)
  useEffect(() => {
    const handleAuthFailure = () => {
      logout();
      router.replace('/login');
    };
    window.addEventListener('auth:logout-required', handleAuthFailure);
    return () => window.removeEventListener('auth:logout-required', handleAuthFailure);
  }, [logout, router]);

  // Clock-driven keepalive — same single-flight redeem as boot / 401.
  useEffect(() => {
    if (!requiresAuth || !hasSession || expiresAt == null) return;
    return scheduleAccessTokenRefresh(expiresAt, () =>
      restoreSessionFromRefreshCookie()
    );
  }, [requiresAuth, hasSession, expiresAt]);

  // Background tab → visible: redeem if soft-expired / missing access (timer may
  // have been throttled while hidden). Boot restore defers while hidden.
  useEffect(() => {
    if (!requiresAuth) return;
    const onVisible = () => {
      if (document.visibilityState !== 'visible') return;
      const state = useAuthStore.getState();
      const sessionOk =
        state.isAuthenticated && !!state.accessToken && !state.isTokenExpired();
      if (sessionOk) return;
      if (pathname === '/login') return;
      void restoreSessionFromRefreshCookie();
    };
    document.addEventListener('visibilitychange', onVisible);
    return () => document.removeEventListener('visibilitychange', onVisible);
  }, [requiresAuth, pathname]);

  // Cold boot, hard refresh, or mid-session soft-expiry: redeem `eq_refresh`.
  // Single-flight lives in restoreSessionFromRefreshCookie (rotate-safe).
  // While the tab is hidden, defer to visibilitychange (background timer skew).
  useEffect(() => {
    if (!hasHydrated || !requiresAuth) {
      return;
    }
    if (hasSession) {
      probing.current = false;
      redirecting.current = false;
      setCookieProbe('done');
      return;
    }
    if (pathname === '/login') {
      setCookieProbe('done');
      return;
    }
    if (
      typeof document !== 'undefined' &&
      document.visibilityState !== 'visible'
    ) {
      probing.current = false;
      setCookieProbe('idle');
      return;
    }
    if (redirecting.current || probing.current) {
      return;
    }

    probing.current = true;
    setCookieProbe('probing');
    void (async () => {
      const restored = await restoreSessionFromRefreshCookie();
      if (restored) {
        // Keep probing until hasSession becomes true so we don't double-redeem.
        setCookieProbe('done');
        return;
      }
      probing.current = false;
      redirecting.current = true;
      setCookieProbe('done');
      router.replace('/login');
    })();
  }, [hasHydrated, requiresAuth, hasSession, pathname, router]);

  // Never gate the login route behind the cookie probe (form must render).
  if (
    requiresAuth &&
    pathname !== '/login' &&
    (!hasHydrated || cookieProbe === 'probing' || (cookieProbe === 'idle' && !hasSession))
  ) {
    if (!hasSession && !seenSession.current) {
      return (
        <div className="flex h-full items-center justify-center">
          <div className="text-center">
            <Loader2 className="mx-auto mb-3 h-8 w-8 animate-spin text-muted-foreground" />
            <p className="text-sm text-muted-foreground">Checking session...</p>
          </div>
        </div>
      );
    }
  }

  // After a failed restore we redirect; avoid a permanent blank shell.
  // Login page must still render children so the password form is usable.
  if (requiresAuth && !hasSession && pathname !== '/login') {
    // Soft expiry: refresh is in flight. Unmounting drops in-progress UI (streams).
    if (seenSession.current && cookieProbe !== 'done') {
      return <>{children}</>;
    }
    return null;
  }

  return <>{children}</>;
}

export default AuthGuard;

/**
 * @module Header
 * @description Application header with status, theme, and user controls.
 * Shows backend connection status, theme toggle, and auth state.
 *
 * @implements FEAT0611 - Backend health indicator in header
 * @implements FEAT0612 - Theme toggle (light/dark/system)
 * @implements FEAT0613 - User menu with logout
 * @implements FEAT0861 - Tenant/workspace selector integration
 * @implements SPEC-155 W7 — ConnectionIndicator SSOT
 */
'use client';

import { ClientOnly } from '@/components/client-only';
import { LanguageSelector } from '@/components/shared/language-selector';
import {
  ConnectionIndicator,
  type ConnectionState,
} from '@/components/shared/connection-indicator';
import { Button } from '@/components/ui/button';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuLabel,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { getBackendReadinessSnapshot } from '@/lib/api/client';
import { getBackendReadyRefetchIntervalForState } from '@/lib/runtime/health-poll';
import { useAuthStore } from '@/stores/use-auth-store';
import { LogOut, Monitor, Moon, Sun, User } from 'lucide-react';
import { useTheme } from 'next-themes';
import { useRouter } from 'next/navigation';
import { useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { useQuery } from '@tanstack/react-query';
import { HeaderSearchTrigger } from './header-search-trigger';
import { HeaderTenantSelector } from './header-tenant-selector';
import { MobileSidebar } from './sidebar';

function mapReadinessToConnection(
  state: string | undefined,
  isLoading: boolean,
): ConnectionState {
  if (isLoading && !state) return 'checking';
  if (state === 'ready') return 'online';
  if (state === 'degraded') return 'degraded';
  if (state === 'unreachable' || state === 'misconfigured') return 'offline';
  return 'checking';
}

export function Header() {
  const { setTheme } = useTheme();
  const router = useRouter();
  const { t } = useTranslation();
  const { isAuthenticated, user, logout } = useAuthStore();

  const { data: readiness, isLoading } = useQuery({
    queryKey: ['backend-ready'],
    queryFn: () => getBackendReadinessSnapshot(),
    refetchInterval: (query) =>
      getBackendReadyRefetchIntervalForState(query.state.data?.state),
    staleTime: 5_000,
    retry: 1,
  });

  const state = readiness?.state;
  const connectionState = mapReadinessToConnection(state, isLoading);
  const version = readiness?.version ?? '';

  const handleThemeChange = useCallback((theme: string) => {
    document.documentElement.classList.add('theme-switching');
    setTheme(theme);
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        document.documentElement.classList.remove('theme-switching');
      });
    });
  }, [setTheme]);

  const handleLogout = () => {
    logout();
    router.push('/login');
  };

  const connectionLabel =
    connectionState === 'online'
      ? t('header.apiVersion', { version: version || '…' })
      : connectionState === 'degraded'
        ? t('header.apiBusy', 'Busy')
        : connectionState === 'offline'
          ? t('connection.offline', 'Offline')
          : t('connection.checking', 'Checking…');

  const connectionHint =
    connectionState === 'online'
      ? t('header.apiVersion', { version: version || '…' })
      : connectionState === 'degraded'
        ? t('header.apiBusyHint', 'API is busy — document counts may lag')
        : connectionState === 'offline'
          ? t('header.apiOfflineHint', 'Cannot connect to EdgeQuake API')
          : t('header.apiCheckingHint', 'Checking connection...');

  return (
    <header className="flex h-12 min-w-0 items-center gap-2 overflow-hidden border-b bg-card/95 backdrop-blur-sm px-3 shrink-0">
      <div className="flex min-w-0 items-center gap-3">
        <MobileSidebar />
        <span className="text-base font-semibold md:hidden" aria-hidden="true">
          EdgeQuake
        </span>

        <div className="hidden md:flex">
          <ClientOnly fallback={null}>
            <HeaderTenantSelector />
          </ClientOnly>
        </div>
      </div>

      <div className="flex min-w-0 flex-1 justify-end lg:justify-center">
        <HeaderSearchTrigger />
      </div>

      <div className="flex shrink-0 items-center gap-1">
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <div className="min-w-[8.5rem] rounded-md px-2 py-1 transition-colors hover:bg-muted/50">
                <ConnectionIndicator state={connectionState} label={connectionLabel} />
              </div>
            </TooltipTrigger>
            <TooltipContent>{connectionHint}</TooltipContent>
          </Tooltip>
        </TooltipProvider>

        <ClientOnly fallback={null}>
          <LanguageSelector />
        </ClientOnly>

        <ClientOnly
          fallback={
            <Button variant="ghost" size="icon" className="h-8 w-8">
              <Sun className="h-4 w-4" />
            </Button>
          }
        >
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon" className="h-8 w-8">
                <Sun className="h-4 w-4 rotate-0 scale-100 transition-all dark:-rotate-90 dark:scale-0" />
                <Moon className="absolute h-4 w-4 rotate-90 scale-0 transition-all dark:rotate-0 dark:scale-100" />
                <span className="sr-only">{t('header.toggleTheme', 'Toggle theme')}</span>
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onClick={() => handleThemeChange('light')}>
                <Sun className="mr-2 h-4 w-4" />
                {t('settings.appearance.themeLight', 'Light')}
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => handleThemeChange('dark')}>
                <Moon className="mr-2 h-4 w-4" />
                {t('settings.appearance.themeDark', 'Dark')}
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => handleThemeChange('system')}>
                <Monitor className="mr-2 h-4 w-4" />
                {t('settings.appearance.themeSystem', 'System')}
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </ClientOnly>

        <ClientOnly
          fallback={
            <Button variant="ghost" size="icon" className="h-8 w-8">
              <User className="h-4 w-4" />
            </Button>
          }
        >
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon" className="h-8 w-8">
                <User className="h-4 w-4" />
                <span className="sr-only">{t('header.userMenu', 'User menu')}</span>
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              {isAuthenticated && user ? (
                <>
                  <DropdownMenuLabel>
                    <div className="flex flex-col">
                      <span>{user.username}</span>
                      {user.email && (
                        <span className="text-xs text-muted-foreground">{user.email}</span>
                      )}
                    </div>
                  </DropdownMenuLabel>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem onClick={handleLogout}>
                    <LogOut className="mr-2 h-4 w-4" />
                    {t('header.logout', 'Logout')}
                  </DropdownMenuItem>
                </>
              ) : (
                <DropdownMenuItem onClick={() => router.push('/login')}>
                  <User className="mr-2 h-4 w-4" />
                  {t('header.signIn', 'Sign In')}
                </DropdownMenuItem>
              )}
            </DropdownMenuContent>
          </DropdownMenu>
        </ClientOnly>
      </div>
    </header>
  );
}

export default Header;

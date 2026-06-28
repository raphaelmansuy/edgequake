/**
 * @module Header
 * @description Application header with status, theme, and user controls.
 * Shows backend connection status, theme toggle, and auth state.
 * 
 * @implements FEAT0611 - Backend health indicator in header
 * @implements FEAT0612 - Theme toggle (light/dark/system)
 * @implements FEAT0613 - User menu with logout
 * @implements FEAT0861 - Tenant/workspace selector integration
 * 
 * @enforces BR0608 - Connection status updates in real-time
 * @enforces BR0609 - Theme persists across sessions
 * 
 * @see {@link docs/features.md} FEAT0611-0613
 */
'use client';

import { ClientOnly } from '@/components/client-only';
import { LanguageSelector } from '@/components/shared/language-selector';
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
import { checkHealth } from '@/lib/api/edgequake';
import { getAutomationAwareRefetchInterval } from '@/lib/runtime/browser-detection';
import { useAuthStore } from '@/stores/use-auth-store';
import { Circle, LogOut, Monitor, Moon, Sun, User } from 'lucide-react';
import { useTheme } from 'next-themes';
import { useRouter } from 'next/navigation';
import { useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { useQuery } from '@tanstack/react-query';
import { HeaderTenantSelector } from './header-tenant-selector';
import { MobileSidebar } from './sidebar';

export function Header() {
  const { setTheme } = useTheme();
  const router = useRouter();
  const { t } = useTranslation();
  const { isAuthenticated, user, logout } = useAuthStore();

  // PP-02: Use shared React Query cache (queryKey: ['health']) instead of a
  // custom setInterval. BackendStatusBanner and SystemStatus card share the
  // same cache entry so only ONE network request is issued every 30 seconds.
  const { data: health, isError } = useQuery({
    queryKey: ['health'],
    queryFn: checkHealth,
    refetchInterval: getAutomationAwareRefetchInterval(30_000),
    staleTime: 15_000,
    retry: 1,
  });

  const connectionStatus = isError ? 'disconnected' : health ? 'connected' : 'checking';
  const version = health?.version ?? '';

  // Smooth theme transition — briefly disables transitions to prevent color flash
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

  return (
    <header className="flex h-12 items-center justify-between border-b bg-card/95 backdrop-blur-sm px-3 shrink-0">
      <div className="flex items-center gap-3">
        <MobileSidebar />
        <span className="text-base font-semibold md:hidden" aria-hidden="true">EdgeQuake</span>
        
        {/* Tenant/Workspace Selector - Desktop only */}
        <div className="hidden md:flex">
          <ClientOnly fallback={null}>
            <HeaderTenantSelector />
          </ClientOnly>
        </div>
      </div>

      <div className="flex items-center gap-1">
        {/* Connection Status */}
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <div className="flex items-center gap-1.5 text-xs text-muted-foreground px-2 py-1 rounded-md hover:bg-muted/50 transition-colors">
                <Circle
                  className={`h-1.5 w-1.5 fill-current ${
                    connectionStatus === 'connected'
                      ? 'text-green-500'
                      : connectionStatus === 'disconnected'
                      ? 'text-red-500'
                      : 'text-yellow-500 animate-pulse'
                  }`}
                />
                <span className="hidden sm:inline font-medium">
                  {connectionStatus === 'connected'
                    ? t('header.apiVersion', { version })
                    : connectionStatus === 'disconnected'
                    ? 'Offline'
                    : '...'}
                </span>
              </div>
            </TooltipTrigger>
            <TooltipContent>
              {connectionStatus === 'connected'
                ? t('header.apiVersion', { version })
                : connectionStatus === 'disconnected'
                ? 'Cannot connect to EdgeQuake API'
                : 'Checking connection...'}
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>

        {/* Language Selector */}
        <ClientOnly fallback={null}>
          <LanguageSelector />
        </ClientOnly>

        {/* Theme Toggle */}
        <ClientOnly fallback={<Button variant="ghost" size="icon" className="h-8 w-8"><Sun className="h-4 w-4" /></Button>}>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon" className="h-8 w-8">
                <Sun className="h-4 w-4 rotate-0 scale-100 transition-all dark:-rotate-90 dark:scale-0" />
                <Moon className="absolute h-4 w-4 rotate-90 scale-0 transition-all dark:rotate-0 dark:scale-100" />
                <span className="sr-only">Toggle theme</span>
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onClick={() => handleThemeChange('light')}>
                <Sun className="mr-2 h-4 w-4" />
                Light
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => handleThemeChange('dark')}>
                <Moon className="mr-2 h-4 w-4" />
                Dark
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => handleThemeChange('system')}>
                <Monitor className="mr-2 h-4 w-4" />
                System
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </ClientOnly>

        {/* User Menu */}
        <ClientOnly fallback={<Button variant="ghost" size="icon" className="h-8 w-8"><User className="h-4 w-4" /></Button>}>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon" className="h-8 w-8">
                <User className="h-4 w-4" />
                <span className="sr-only">User menu</span>
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
                    Logout
                  </DropdownMenuItem>
                </>
              ) : (
                <DropdownMenuItem onClick={() => router.push('/login')}>
                  <User className="mr-2 h-4 w-4" />
                  Sign In
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

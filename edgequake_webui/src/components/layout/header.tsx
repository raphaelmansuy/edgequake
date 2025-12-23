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
import { useAuthStore } from '@/stores/use-auth-store';
import { Circle, LogOut, Monitor, Moon, Sun, User } from 'lucide-react';
import { useTheme } from 'next-themes';
import { useRouter } from 'next/navigation';
import { useEffect, useState } from 'react';
import { MobileSidebar } from './sidebar';

type ConnectionStatus = 'connected' | 'disconnected' | 'checking';

export function Header() {
  const { setTheme } = useTheme();
  const router = useRouter();
  const { isAuthenticated, user, logout } = useAuthStore();
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>('checking');
  const [version, setVersion] = useState<string>('');

  // Check backend connection status
  useEffect(() => {
    const checkConnection = async () => {
      try {
        const health = await checkHealth();
        setConnectionStatus('connected');
        setVersion(health.version || '');
      } catch {
        setConnectionStatus('disconnected');
      }
    };

    checkConnection();
    const interval = setInterval(checkConnection, 30000); // Check every 30s
    return () => clearInterval(interval);
  }, []);

  const handleLogout = () => {
    logout();
    router.push('/login');
  };

  return (
    <header className="flex h-16 items-center justify-between border-b bg-card px-4">
      <div className="flex items-center gap-4">
        <MobileSidebar />
        <h1 className="text-lg font-semibold md:hidden">EdgeQuake</h1>
      </div>

      <div className="flex items-center gap-2">
        {/* Connection Status */}
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <div className="flex items-center gap-1.5 text-sm text-muted-foreground px-2">
                <Circle
                  className={`h-2 w-2 fill-current ${
                    connectionStatus === 'connected'
                      ? 'text-green-500'
                      : connectionStatus === 'disconnected'
                      ? 'text-red-500'
                      : 'text-yellow-500 animate-pulse'
                  }`}
                />
                <span className="hidden sm:inline">
                  {connectionStatus === 'connected'
                    ? `API ${version}`
                    : connectionStatus === 'disconnected'
                    ? 'Offline'
                    : 'Connecting...'}
                </span>
              </div>
            </TooltipTrigger>
            <TooltipContent>
              {connectionStatus === 'connected'
                ? `Connected to EdgeQuake API v${version}`
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
        <ClientOnly fallback={<Button variant="ghost" size="icon"><Sun className="h-5 w-5" /></Button>}>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon">
                <Sun className="h-5 w-5 rotate-0 scale-100 transition-all dark:-rotate-90 dark:scale-0" />
                <Moon className="absolute h-5 w-5 rotate-90 scale-0 transition-all dark:rotate-0 dark:scale-100" />
                <span className="sr-only">Toggle theme</span>
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onClick={() => setTheme('light')}>
                <Sun className="mr-2 h-4 w-4" />
                Light
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => setTheme('dark')}>
                <Moon className="mr-2 h-4 w-4" />
                Dark
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => setTheme('system')}>
                <Monitor className="mr-2 h-4 w-4" />
                System
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </ClientOnly>

        {/* User Menu */}
        <ClientOnly fallback={<Button variant="ghost" size="icon"><User className="h-5 w-5" /></Button>}>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon">
                <User className="h-5 w-5" />
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

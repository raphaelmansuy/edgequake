/**
 * @module AppProviders
 * @description Root provider composition for EdgeQuake WebUI.
 * Wraps application with theme, i18n, query, and WebSocket providers.
 *
 * @implements FEAT0860 - Provider composition pattern
 * @implements FEAT0861 - Context layering for global state
 *
 * @enforces BR0860 - Provider order maintained for dependencies
 */
'use client';

import { Toaster } from '@/components/ui/sonner';
import { type ReactNode } from 'react';
import { I18nProvider } from './i18n-provider';
import { KeyboardShortcutsProvider } from './keyboard-shortcuts-provider';
import { QueryProvider } from './query-provider';
import { TenantProvider } from './tenant-provider';
import { ThemeProvider } from './theme-provider';
import { WebSocketProvider } from './websocket-provider';

interface AppProvidersProps {
  children: ReactNode;
}

/**
 * Root provider that wraps the entire application.
 * 
 * Provider order is important:
 * 1. QueryProvider - React Query for server state
 * 2. ThemeProvider - Theme must be available early to prevent flash
 * 3. I18nProvider - Internationalization
 * 4. TenantProvider - Tenant/workspace context initialization (handles hydration internally)
 * 5. WebSocketProvider - Real-time updates
 * 6. KeyboardShortcutsProvider - Keyboard shortcuts
 * 
 * Note: HydrationProvider is available but not used in the main hierarchy
 * because TenantGuard and individual stores handle hydration states.
 * Use HydrationProvider if you need app-wide hydration gating.
 */
export function AppProviders({ children }: AppProvidersProps) {
  return (
    <QueryProvider>
      <ThemeProvider>
        <I18nProvider>
          <TenantProvider>
            <WebSocketProvider>
              <KeyboardShortcutsProvider>
                {children}
                <Toaster 
                  richColors 
                  position="bottom-right" 
                  duration={3000}
                  closeButton
                />
              </KeyboardShortcutsProvider>
            </WebSocketProvider>
          </TenantProvider>
        </I18nProvider>
      </ThemeProvider>
    </QueryProvider>
  );
}

export { HydrationProvider } from './hydration-provider';
export { TenantProvider } from './tenant-provider';
export { WebSocketProvider } from './websocket-provider';
export default AppProviders;

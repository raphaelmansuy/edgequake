'use client';

import { Toaster } from '@/components/ui/sonner';
import { type ReactNode } from 'react';
import { I18nProvider } from './i18n-provider';
import { KeyboardShortcutsProvider } from './keyboard-shortcuts-provider';
import { QueryProvider } from './query-provider';
import { ThemeProvider } from './theme-provider';

interface AppProvidersProps {
  children: ReactNode;
}

export function AppProviders({ children }: AppProvidersProps) {
  return (
    <QueryProvider>
      <ThemeProvider>
        <I18nProvider>
          <KeyboardShortcutsProvider>
            {children}
            <Toaster richColors position="bottom-right" />
          </KeyboardShortcutsProvider>
        </I18nProvider>
      </ThemeProvider>
    </QueryProvider>
  );
}

export default AppProviders;

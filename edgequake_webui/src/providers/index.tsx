'use client';

import { Toaster } from '@/components/ui/sonner';
import { type ReactNode } from 'react';
import { QueryProvider } from './query-provider';
import { ThemeProvider } from './theme-provider';

interface AppProvidersProps {
  children: ReactNode;
}

export function AppProviders({ children }: AppProvidersProps) {
  return (
    <QueryProvider>
      <ThemeProvider>
        {children}
        <Toaster richColors position="bottom-right" />
      </ThemeProvider>
    </QueryProvider>
  );
}

export default AppProviders;

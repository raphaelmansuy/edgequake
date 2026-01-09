/**
 * @module QueryProvider
 * @description React Query provider with default configuration.
 *
 * @implements FEAT0863 - Server state management with React Query
 * @implements FEAT0864 - Automatic cache invalidation
 *
 * @enforces BR0863 - Stale time 1 minute for fresh data
 * @enforces BR0864 - Single retry on failure
 */
'use client';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useState, type ReactNode } from 'react';

interface QueryProviderProps {
  children: ReactNode;
}

export function QueryProvider({ children }: QueryProviderProps) {
  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            staleTime: 60 * 1000, // 1 minute
            gcTime: 5 * 60 * 1000, // 5 minutes (previously cacheTime)
            retry: 1,
            refetchOnWindowFocus: false,
          },
          mutations: {
            retry: 0,
          },
        },
      })
  );

  return (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

export default QueryProvider;

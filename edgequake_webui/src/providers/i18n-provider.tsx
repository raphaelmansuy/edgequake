'use client';

import '@/lib/i18n';
import { useEffect, useState } from 'react';

interface I18nProviderProps {
  children: React.ReactNode;
}

/**
 * I18n Provider component that ensures i18n is properly initialized
 * before rendering children. This prevents hydration mismatches
 * between server and client.
 */
export function I18nProvider({ children }: I18nProviderProps) {
  const [isHydrated, setIsHydrated] = useState(false);

  useEffect(() => {
    setIsHydrated(true);
  }, []);

  // On first render (SSR), we return null to prevent hydration mismatch
  // Once client hydrates, we render children
  if (!isHydrated) {
    return null;
  }

  return <>{children}</>;
}

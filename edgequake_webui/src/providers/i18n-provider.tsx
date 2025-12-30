'use client';

import '@/lib/i18n';
import { useSyncExternalStore } from 'react';

interface I18nProviderProps {
  children: React.ReactNode;
}

// Hydration detection using useSyncExternalStore pattern
// This is SSR-safe and follows React 18+ best practices

// Store for tracking subscribers (enables reactive updates if needed)
const subscribers = new Set<() => void>();
let hydrationState = false;

// Initialize hydration on client-side (runs once when module loads)
if (typeof window !== 'undefined') {
  hydrationState = true;
}

function subscribe(callback: () => void): () => void {
  subscribers.add(callback);
  return () => subscribers.delete(callback);
}

function getSnapshot(): boolean {
  return hydrationState;
}

function getServerSnapshot(): boolean {
  return false;
}

/**
 * I18n Provider component that ensures i18n is properly initialized
 * before rendering children. This prevents hydration mismatches
 * between server and client.
 *
 * Uses useSyncExternalStore for SSR-safe hydration detection.
 */
export function I18nProvider({ children }: I18nProviderProps) {
  const hydrated = useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot);

  // On first render (SSR), we return null to prevent hydration mismatch
  // Once client hydrates, we render children
  if (!hydrated) {
    return null;
  }

  return <>{children}</>;
}

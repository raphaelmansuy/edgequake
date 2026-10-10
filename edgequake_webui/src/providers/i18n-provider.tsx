/**
 * @module I18nProvider
 * @description Internationalization provider with SSR-safe hydration.
 *
 * @implements FEAT0729 - Multi-language support (en, zh, fr)
 * @implements FEAT0867 - SSR-safe i18n hydration
 *
 * @enforces BR0726 - Fallback to English for missing keys
 * @enforces BR0867 - No hydration mismatch for i18n
 */
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

  // Server and the hydration pass share this shell so the first paint is not
  // blank. Geometry matches the dashboard chrome (sidebar w-64, header h-12).
  if (!hydrated) {
    return (
      <div
        className="flex h-dvh max-h-dvh overflow-hidden bg-background"
        aria-busy="true"
        data-testid="app-shell-skeleton"
      >
        <div className="hidden w-64 shrink-0 border-r bg-card md:block" />
        <div className="flex min-w-0 flex-1 flex-col">
          <div className="h-12 shrink-0 border-b bg-card" />
          <div className="flex-1" />
        </div>
      </div>
    );
  }

  return <>{children}</>;
}

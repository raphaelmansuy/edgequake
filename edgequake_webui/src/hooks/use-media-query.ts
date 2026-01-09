"use client";

import { useSyncExternalStore } from "react";

/**
 * @module use-media-query
 * @description Hook to check if a media query matches.
 * Uses useSyncExternalStore for proper SSR and hydration handling.
 *
 * @implements FEAT0641 - Responsive breakpoint detection
 * @implements FEAT0642 - SSR-safe media query matching
 *
 * @enforces BR0627 - Server renders mobile-first layout
 */

/**
 * Get current match status for a media query (for SSR fallback)
 */
function getServerSnapshot(): boolean {
  return false; // Default to false on server
}

/**
 * Hook to check if a media query matches.
 * Uses useSyncExternalStore for proper SSR and hydration handling.
 *
 * @param query - CSS media query string (e.g., '(max-width: 768px)')
 * @returns boolean indicating if the query matches
 *
 * @example
 * ```tsx
 * const isMobile = useMediaQuery('(max-width: 768px)');
 * const prefersDark = useMediaQuery('(prefers-color-scheme: dark)');
 * ```
 */
export function useMediaQuery(query: string): boolean {
  const getSnapshot = () => {
    if (typeof window === "undefined") {
      return false;
    }
    return window.matchMedia(query).matches;
  };

  const subscribe = (callback: () => void) => {
    if (typeof window === "undefined") {
      return () => {};
    }
    const media = window.matchMedia(query);
    media.addEventListener("change", callback);
    return () => media.removeEventListener("change", callback);
  };

  return useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot);
}

export default useMediaQuery;

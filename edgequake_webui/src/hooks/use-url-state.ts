"use client";

import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { useCallback, useMemo } from "react";

/**
 * @module use-url-state
 * @description A hook for syncing state with URL search parameters.
 * This enables shareable URLs with filters, pagination, and other state.
 *
 * @implements FEAT0647 - URL state synchronization
 * @implements FEAT0648 - Shareable filter URLs
 *
 * @enforces BR0632 - URL params validated before applying
 * @enforces BR0633 - Invalid params use defaults
 */

type StateValue = string | number | boolean | undefined;

export interface UrlStateConfig<T extends Record<string, StateValue>> {
  defaultValues: T;
  scroll?: boolean;
}

/**
 * A hook for syncing state with URL search parameters.
 * This enables shareable URLs with filters, pagination, and other state.
 */
export function useUrlState<T extends Record<string, StateValue>>(
  config: UrlStateConfig<T>
): {
  state: T;
  setState: (updates: Partial<T>) => void;
  resetState: () => void;
} {
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const { defaultValues, scroll = false } = config;

  // Parse current URL state
  const state = useMemo((): T => {
    const result = { ...defaultValues };

    for (const key of Object.keys(defaultValues)) {
      const value = searchParams.get(key);
      if (value !== null) {
        const defaultValue = defaultValues[key];

        // Type coercion based on default value type
        if (typeof defaultValue === "number") {
          const parsed = Number(value);
          if (!isNaN(parsed)) {
            (result as Record<string, StateValue>)[key] = parsed;
          }
        } else if (typeof defaultValue === "boolean") {
          (result as Record<string, StateValue>)[key] = value === "true";
        } else {
          (result as Record<string, StateValue>)[key] = value;
        }
      }
    }

    return result;
  }, [searchParams, defaultValues]);

  // Update URL with new state
  const setState = useCallback(
    (updates: Partial<T>) => {
      const params = new URLSearchParams(searchParams.toString());

      for (const [key, value] of Object.entries(updates)) {
        if (value === undefined || value === defaultValues[key]) {
          // Remove from URL if undefined or equals default
          params.delete(key);
        } else {
          params.set(key, String(value));
        }
      }

      const queryString = params.toString();
      const url = queryString ? `${pathname}?${queryString}` : pathname;
      router.push(url, { scroll });
    },
    [router, pathname, searchParams, defaultValues, scroll]
  );

  // Reset to defaults
  const resetState = useCallback(() => {
    router.push(pathname, { scroll });
  }, [router, pathname, scroll]);

  return { state, setState, resetState };
}

/**
 * A simplified hook for a single URL parameter.
 */
export function useUrlParam<T extends StateValue>(
  key: string,
  defaultValue: T
): [T, (value: T | undefined) => void] {
  const { state, setState } = useUrlState({
    defaultValues: { [key]: defaultValue } as Record<string, T>,
  });

  const value = state[key] as T;
  const setValue = useCallback(
    (newValue: T | undefined) => {
      setState({ [key]: newValue } as Partial<Record<string, T>>);
    },
    [setState, key]
  );

  return [value, setValue];
}

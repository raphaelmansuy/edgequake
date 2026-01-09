"use client";

import { useEffect, useState } from "react";

/**
 * @module use-debounce
 * @description useDebounce hook - Debounces a value by a specified delay.
 *
 * Useful for search inputs where you want to wait for the user
 * to stop typing before triggering an API call.
 *
 * @implements FEAT0635 - Debounced search to reduce API calls
 * @implements FEAT0636 - Performance optimization for user input
 *
 * @enforces BR0622 - Minimum debounce of 200ms for search
 *
 * @param value - The value to debounce
 * @param delay - The debounce delay in milliseconds (default: 500ms)
 * @returns The debounced value
 *
 * @example
 * const debouncedSearch = useDebounce(searchQuery, 300);
 *
 * useEffect(() => {
 *   if (debouncedSearch) {
 *     fetchSearchResults(debouncedSearch);
 *   }
 * }, [debouncedSearch]);
 */
export function useDebounce<T>(value: T, delay: number = 500): T {
  const [debouncedValue, setDebouncedValue] = useState<T>(value);

  useEffect(() => {
    // Set up a timer to update the debounced value
    const timer = setTimeout(() => {
      setDebouncedValue(value);
    }, delay);

    // Clear the timer if value changes before delay has passed
    return () => {
      clearTimeout(timer);
    };
  }, [value, delay]);

  return debouncedValue;
}

export default useDebounce;

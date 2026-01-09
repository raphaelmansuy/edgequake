/**
 * @module ThemeProvider
 * @description Theme context provider using next-themes.
 *
 * @implements FEAT0800 - Theme support (light/dark/system)
 *
 * @enforces BR0800 - Theme persisted in localStorage
 * @enforces BR0862 - No transition flash on theme change
 */
'use client';

import type { ThemeProviderProps } from 'next-themes';
import { ThemeProvider as NextThemesProvider } from 'next-themes';

export function ThemeProvider({ children, ...props }: ThemeProviderProps) {
  return (
    <NextThemesProvider
      attribute="class"
      defaultTheme="system"
      enableSystem
      disableTransitionOnChange
      {...props}
    >
      {children}
    </NextThemesProvider>
  );
}

export default ThemeProvider;

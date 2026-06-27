/**
 * @module RootLayout
 * @description Root layout component for EdgeQuake WebUI.
 * Provides theme, i18n, and React Query providers.
 *
 * @implements FEAT0800 - Theme support (light/dark/system)
 * @implements FEAT0729 - Multi-language support
 *
 * @enforces BR0800 - Theme persisted in localStorage
 */
import { getRuntimeConfig } from '@/lib/runtime-config';
import { resolveRuntimeApiUrlForInjection } from '@/lib/server/resolve-runtime-api-url';
import { AppProviders } from '@/providers';
import type { Metadata } from 'next';
import { Inter } from 'next/font/google';
import './globals.css';

// Force per-request rendering so getRuntimeConfig() reads container env vars
// (EDGEQUAKE_API_URL, NEXT_PUBLIC_AUTH_ENABLED, NEXT_PUBLIC_DISABLE_DEMO_LOGIN)
// at request time instead of baking build-time defaults into a static HTML shell.
// @implements SPEC-013 / GitHub #218
export const dynamic = 'force-dynamic';

const inter = Inter({
  variable: '--font-inter',
  subsets: ['latin'],
});

export const metadata: Metadata = {
  title: 'EdgeQuake - Knowledge Graph RAG Platform',
  description: 'Advanced Retrieval-Augmented Generation with graph-based knowledge representation',
  keywords: ['RAG', 'Knowledge Graph', 'LLM', 'AI', 'Graph Database'],
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  const runtimeConfig = {
    ...getRuntimeConfig(),
    // P-G13: dev uses same-origin rewrites; prod/docker inject absolute apiUrl.
    apiUrl: resolveRuntimeApiUrlForInjection() || getRuntimeConfig().apiUrl,
  };

  return (
    <html lang="en" suppressHydrationWarning>
      <body className={`${inter.variable} font-sans antialiased`} suppressHydrationWarning>
        <script
          suppressHydrationWarning
          dangerouslySetInnerHTML={{
            __html: `window.__EDGEQUAKE_RUNTIME_CONFIG__ = ${JSON.stringify(runtimeConfig)};`,
          }}
        />
        <AppProviders>{children}</AppProviders>
      </body>
    </html>
  );
}

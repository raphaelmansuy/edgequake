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
import { AppProviders } from '@/providers';
import type { Metadata } from 'next';
import { Inter } from 'next/font/google';
import './globals.css';

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
  return (
    <html lang="en" suppressHydrationWarning>
      <body className={`${inter.variable} font-sans antialiased`} suppressHydrationWarning>
        <AppProviders>{children}</AppProviders>
      </body>
    </html>
  );
}

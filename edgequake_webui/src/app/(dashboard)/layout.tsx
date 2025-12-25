'use client';

import { DynamicBreadcrumb } from '@/components/layout/dynamic-breadcrumb';
import { Header } from '@/components/layout/header';
import { Sidebar } from '@/components/layout/sidebar';
import { SkipLink } from '@/components/shared/skip-link';
import { useKeyboardShortcuts } from '@/hooks/use-keyboard-shortcuts';

export default function DashboardLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  // Enable global keyboard shortcuts
  useKeyboardShortcuts();

  return (
    <div className="flex h-screen overflow-hidden bg-background">
      <SkipLink />
      <Sidebar />
      <div className="flex flex-1 flex-col overflow-hidden">
        <Header />
        {/* Breadcrumb Navigation - consistent padding with header */}
        <div className="border-b px-6 py-3 bg-muted/30">
          <DynamicBreadcrumb />
        </div>
        <main 
          id="main-content" 
          className="flex-1 overflow-auto scroll-smooth" 
          tabIndex={-1}
        >
          {children}
        </main>
      </div>
    </div>
  );
}

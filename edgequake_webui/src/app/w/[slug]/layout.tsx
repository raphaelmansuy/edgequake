'use client';

import { DynamicBreadcrumb } from '@/components/layout/dynamic-breadcrumb';
import { Header } from '@/components/layout/header';
import { Sidebar } from '@/components/layout/sidebar';
import { TenantGuard } from '@/components/layout/tenant-guard';
import { SkipLink } from '@/components/shared/skip-link';
import { useKeyboardShortcuts } from '@/hooks/use-keyboard-shortcuts';

/**
 * Layout for workspace deeplink routes.
 * 
 * @implements SPEC-032: Focus 6 - Deeplinks to workspace
 * 
 * Uses same layout as dashboard for consistent UX.
 */
export default function WorkspaceDeeplinkLayout({
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
        {/* Breadcrumb Navigation - compact */}
        <div className="border-b px-4 py-2 bg-muted/20">
          <DynamicBreadcrumb />
        </div>
        {/* Main content area */}
        <main 
          id="main-content" 
          className="flex-1 min-h-0 overflow-hidden" 
          tabIndex={-1}
        >
          <TenantGuard>
            {children}
          </TenantGuard>
        </main>
      </div>
    </div>
  );
}

'use client';

import { DynamicBreadcrumb } from '@/components/layout/dynamic-breadcrumb';
import { Header } from '@/components/layout/header';
import { Sidebar } from '@/components/layout/sidebar';
import { SkipLink } from '@/components/shared/skip-link';
import { useKeyboardShortcuts } from '@/hooks/use-keyboard-shortcuts';

/**
 * Layout for workspace deeplink routes.
 * 
 * @implements SPEC-032: Focus 6 - Deeplinks to workspace
 * @iteration OODA 61 - Removed TenantGuard wrapper to fix race condition
 * 
 * Uses same layout as dashboard for consistent UX.
 * 
 * Note: TenantGuard was removed because deeplink pages handle their own
 * workspace resolution and loading/error states. Having TenantGuard here
 * caused a race condition where it would show "Create Workspace" before
 * the deeplink page could set the workspace context.
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
        {/* Main content area - no TenantGuard (pages handle their own context) */}
        <main 
          id="main-content" 
          className="flex-1 min-h-0 overflow-hidden" 
          tabIndex={-1}
        >
          {children}
        </main>
      </div>
    </div>
  );
}

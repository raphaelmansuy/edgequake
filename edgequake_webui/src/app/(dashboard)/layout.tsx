'use client';

import { DynamicBreadcrumb } from '@/components/layout/dynamic-breadcrumb';
import { Header } from '@/components/layout/header';
import { Sidebar } from '@/components/layout/sidebar';
import { TenantGuard } from '@/components/layout/tenant-guard';
import { SkipLink } from '@/components/shared/skip-link';
import { useKeyboardShortcuts } from '@/hooks/use-keyboard-shortcuts';
import { useWorkspaceUrl } from '@/hooks/use-workspace-url';
import { Suspense } from 'react';

// Wrap the workspace URL hook in a component for Suspense boundary
function WorkspaceUrlSync() {
  useWorkspaceUrl();
  return null;
}

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
      {/* Workspace URL sync - wrapped in Suspense for useSearchParams */}
      <Suspense fallback={null}>
        <WorkspaceUrlSync />
      </Suspense>
      <div className="flex flex-1 flex-col overflow-hidden">
        <Header />
        {/* Breadcrumb Navigation - compact */}
        <div className="border-b px-4 py-2 bg-muted/20">
          <DynamicBreadcrumb />
        </div>
        {/* Main content area - each page controls its own scrolling */}
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

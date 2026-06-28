'use client';

import { AuthGuard } from '@/components/layout/auth-guard';
import { DynamicBreadcrumb } from '@/components/layout/dynamic-breadcrumb';
import { Header } from '@/components/layout/header';
import { Sidebar } from '@/components/layout/sidebar';
import { TenantGuard } from '@/components/layout/tenant-guard';
import { ApiErrorBoundary } from '@/components/shared/api-error-boundary';
import { BackendStatusBanner } from '@/components/shared/backend-status-banner';
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
    <AuthGuard>
      <div className="flex h-screen overflow-hidden bg-background">
        <SkipLink />
        <Sidebar />
        {/* Workspace URL sync - wrapped in Suspense for useSearchParams */}
        <Suspense fallback={null}>
          <WorkspaceUrlSync />
        </Suspense>
        <div className="flex flex-1 flex-col overflow-hidden">
          <Header />
          {/* Backend-not-ready banner: fixed overlay, no layout shift (ES-01) */}
          <BackendStatusBanner />
          {/* Breadcrumb: renders its own container; null at depth ≤ 1 (no empty space) */}
          <DynamicBreadcrumb />
          {/* Main content area - each page controls its own scrolling.
              Error boundary isolates render failures (e.g., undefined stats
              fields when the API is unreachable) to this subtree. */}
          <main
            id="main-content"
            className="flex-1 min-h-0 overflow-hidden"
            tabIndex={-1}
          >
            <ApiErrorBoundary>
              <TenantGuard>
                {children}
              </TenantGuard>
            </ApiErrorBoundary>
          </main>
        </div>
      </div>
    </AuthGuard>
  );
}

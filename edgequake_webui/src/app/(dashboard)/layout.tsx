'use client';

import { AuthGuard } from '@/components/layout/auth-guard';
import { DynamicBreadcrumb } from '@/components/layout/dynamic-breadcrumb';
import { Header } from '@/components/layout/header';
import { Sidebar } from '@/components/layout/sidebar';
import { TenantGuard } from '@/components/layout/tenant-guard';
import { FirstRunWizard } from '@/components/onboarding/first-run-wizard';
import { ApiErrorBoundary } from '@/components/shared/api-error-boundary';
import { BreakGlassBanner } from '@/components/security/break-glass-banner';
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
      {/* SPEC-101: post-login first-run when tenants still missing */}
      <FirstRunWizard />
      {/*
        SPEC-099: every flex child on the height chain needs min-h-0. Without it,
        min-height:auto lets the documents virtualizer inflate this column past
        100dvh → document/body scroll + white spacer band under ~N rows.
        overflow-clip stops clipped overflow from extending document.scrollHeight.
      */}
      <div className="flex h-dvh max-h-dvh min-h-0 overflow-clip bg-background">
        <SkipLink />
        <Sidebar />
        {/* Workspace URL sync - wrapped in Suspense for useSearchParams */}
        <Suspense fallback={null}>
          <WorkspaceUrlSync />
        </Suspense>
        <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-clip">
          <Header />
          {/* Backend-not-ready banner: fixed overlay, no layout shift (ES-01) */}
          <BackendStatusBanner />
          {/* SPEC-146: active break-glass session banner (audited elevated access) */}
          <BreakGlassBanner />
          {/* Breadcrumb: null at depth ≤ 1 (no empty band); bar at depth ≥ 2 */}
          <DynamicBreadcrumb />
          {/* Main content area - each page controls its own scrolling.
              Error boundary isolates render failures (e.g., undefined stats
              fields when the API is unreachable) to this subtree. */}
          <main
            id="main-content"
            className="flex min-h-0 flex-1 flex-col overflow-clip"
            tabIndex={-1}
          >
            <ApiErrorBoundary>
              <TenantGuard>
                <div className="flex h-full min-h-0 flex-col overflow-clip">
                  {children}
                </div>
              </TenantGuard>
            </ApiErrorBoundary>
          </main>
        </div>
      </div>
    </AuthGuard>
  );
}

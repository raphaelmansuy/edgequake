'use client';

import { QuickActions, RecentActivity, SystemStatus } from '@/components/dashboard';
import { ScrollArea } from '@/components/ui/scroll-area';
import { getDocuments } from '@/lib/api/edgequake';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useQuery } from '@tanstack/react-query';
import { useRouter, useSearchParams } from 'next/navigation';
import { Suspense, useEffect } from 'react';
import { useTranslation } from 'react-i18next';

// Component to handle URL updates with Suspense boundary
function WorkspaceUrlUpdater() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const { selectedWorkspaceId, workspaces, selectWorkspace } = useTenantStore();

  useEffect(() => {
    const hasWorkspaceParam = searchParams.get('workspace');

    // If no workspace in URL but we have workspaces available
    if (!hasWorkspaceParam && workspaces.length > 0) {
      // Determine which workspace to use
      let targetWorkspace;
      
      if (selectedWorkspaceId) {
        // Use currently selected workspace
        targetWorkspace = workspaces.find(w => w.id === selectedWorkspaceId);
      } else {
        // Auto-select first workspace
        targetWorkspace = workspaces[0];
        selectWorkspace(targetWorkspace.id);
      }
      
      // Update URL with workspace slug
      if (targetWorkspace?.slug) {
        const params = new URLSearchParams(searchParams.toString());
        params.set('workspace', targetWorkspace.slug);
        router.replace(`/?${params.toString()}`, { scroll: false });
      }
    }
  }, [selectedWorkspaceId, workspaces, selectWorkspace, searchParams, router]);

  return null;
}

export default function DashboardPage() {
  const { t } = useTranslation();

  // Get tenant context for query keys
  const { selectedTenantId, selectedWorkspaceId } = useTenantStore();

  // NOTE: Auto-select logic removed - handled by WorkspaceUrlUpdater component
  // to avoid duplicate selection logic and race conditions

  // Fetch recent documents for activity feed
  const { data: documentsData, isLoading: isLoadingDocs } = useQuery({
    queryKey: ['documents', selectedTenantId, selectedWorkspaceId, 1, 10],
    queryFn: () => getDocuments({ page: 1, page_size: 10 }),
    staleTime: 30000,
  });

  const recentDocuments = documentsData?.items || [];

  return (
    <ScrollArea className="h-full">
      {/* URL updater with Suspense boundary for useSearchParams */}
      <Suspense fallback={null}>
        <WorkspaceUrlUpdater />
      </Suspense>
      <div className="p-page space-y-6">
        {/* Header Section - Compact */}
        <header className="space-y-1">
          <h1 className="text-2xl font-bold tracking-tight">
            {t('dashboard.title', 'Dashboard')}
          </h1>
          <p className="text-sm text-muted-foreground max-w-2xl">
            {t('dashboard.welcome', 'Welcome to EdgeQuake - Your Knowledge Graph RAG Platform')}
          </p>
        </header>

        {/* Stats Cards Grid - Responsive gaps */}
        <section aria-label="Statistics" className="grid gap-4 sm:gap-5 lg:gap-6 sm:grid-cols-2 lg:grid-cols-4">
          <StatsCard
            title={t('dashboard.stats.documents', 'Documents')}
            value={documentCount}
            description={t('dashboard.stats.documentsDesc', 'Uploaded documents')}
            icon={FileText}
            isLoading={isLoadingStats}
            variant="documents"
          />
          <StatsCard
            title={t('dashboard.stats.entities', 'Entities')}
            value={entityCount}
            description={t('dashboard.stats.entitiesDesc', 'Extracted entities')}
            icon={Users}
            isLoading={isLoadingStats}
            variant="entities"
          />
          <SArea>
  );
}

'use client';

import { QuickActions, RecentActivity, StatsCard, SystemStatus } from '@/components/dashboard';
import { ScrollArea } from '@/components/ui/scroll-area';
import { getDocuments, getWorkspaceStats } from '@/lib/api/edgequake';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useQuery } from '@tanstack/react-query';
import { FileText, GitMerge, Network, Users } from 'lucide-react';
import { useRouter, useSearchParams } from 'next/navigation';
import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';

export default function DashboardPage() {
  const { t } = useTranslation();
  const router = useRouter();
  const searchParams = useSearchParams();

  // Get tenant context for query keys
  const { selectedTenantId, selectedWorkspaceId, workspaces, selectWorkspace } = useTenantStore();

  // Auto-select first workspace if none selected (e.g., direct navigation to /)
  // WHY: This effect must run when workspaces array changes (not just on mount)
  // because workspaces are loaded asynchronously by TenantGuard
  useEffect(() => {
    if (!selectedWorkspaceId && workspaces.length > 0) {
      const firstWorkspace = workspaces[0];
      selectWorkspace(firstWorkspace.id);
      
      // Update URL to include workspace parameter
      // WHY: Ensures URL reflects selected workspace for sharing and browser history
      const hasWorkspaceParam = searchParams.get('workspace');
      if (!hasWorkspaceParam && firstWorkspace.slug) {
        const params = new URLSearchParams(searchParams.toString());
        params.set('workspace', firstWorkspace.slug);
        router.replace(`/?${params.toString()}`, { scroll: false });
      }
    }
  }, [selectedWorkspaceId, workspaces, selectWorkspace, searchParams, router]);

  // Fetch workspace stats (includes document count, entity count, relationship count)
  const { data: statsData, isLoading: isLoadingStats } = useQuery({
    queryKey: ['workspaceStats', selectedTenantId, selectedWorkspaceId],
    queryFn: () =>
      selectedWorkspaceId
        ? getWorkspaceStats(selectedWorkspaceId)
        : Promise.reject(new Error('No workspace selected')),
    enabled: !!selectedWorkspaceId,
    staleTime: 30000,
  });

  // Fetch recent documents for activity feed
  const { data: documentsData, isLoading: isLoadingDocs } = useQuery({
    queryKey: ['documents', selectedTenantId, selectedWorkspaceId, 1, 10],
    queryFn: () => getDocuments({ page: 1, page_size: 10 }),
    staleTime: 30000,
  });

  const documentCount = statsData?.document_count || 0;
  const entityCount = statsData?.entity_count || 0;
  const relationshipCount = statsData?.relationship_count || 0;
  const recentDocuments = documentsData?.items || [];

  // For entity types, we'll keep this simple for now
  // TODO: Get actual unique entity types from backend
  const entityTypes = entityCount > 0 ? 1 : 0;

  return (
    <ScrollArea className="h-full">
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
          <StatsCard
            title={t('dashboard.stats.relationships', 'Relationships')}
            value={relationshipCount}
            description={t('dashboard.stats.relationshipsDesc', 'Entity connections')}
            icon={GitMerge}
            isLoading={isLoadingStats}
            variant="relationships"
          />
          <StatsCard
            title={t('dashboard.stats.entityTypes', 'Entity Types')}
            value={entityTypes}
            description={t('dashboard.stats.entityTypesDesc', 'Unique categories')}
            icon={Network}
            isLoading={isLoadingStats}
            variant="types"
          />
        </section>

        {/* Quick Actions */}
        <section aria-label="Quick Actions">
          <QuickActions />
        </section>

        {/* Recent Activity and System Status */}
        <section aria-label="Activity and Status" className="grid gap-6 lg:grid-cols-3">
          <div className="lg:col-span-2">
            <RecentActivity 
              documents={recentDocuments} 
              isLoading={isLoadingDocs}
            />
          </div>
          <div>
            <SystemStatus />
          </div>
        </section>
      </div>
    </ScrollArea>
  );
}

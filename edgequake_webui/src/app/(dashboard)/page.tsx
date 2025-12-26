'use client';

import { QuickActions, RecentActivity, StatsCard, SystemStatus } from '@/components/dashboard';
import { ScrollArea } from '@/components/ui/scroll-area';
import { getDocuments, getGraph } from '@/lib/api/edgequake';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useQuery } from '@tanstack/react-query';
import { FileText, GitMerge, Network, Users } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export default function DashboardPage() {
  const { t } = useTranslation();

  // Get tenant context for query keys
  const { selectedTenantId, selectedWorkspaceId } = useTenantStore();

  // Fetch document count
  const { data: documentsData, isLoading: isLoadingDocs } = useQuery({
    queryKey: ['documents', selectedTenantId, selectedWorkspaceId, 1, 10],
    queryFn: () => getDocuments({ page: 1, page_size: 10 }),
    staleTime: 30000,
  });

  // Fetch graph stats
  const { data: graphData, isLoading: isLoadingGraph } = useQuery({
    queryKey: ['graph', selectedTenantId, selectedWorkspaceId],
    queryFn: () => getGraph({ limit: 1 }), // Just need metadata
    staleTime: 30000,
  });

  const documentCount = documentsData?.total || documentsData?.items?.length || 0;
  const entityCount = graphData?.metadata?.node_count || 0;
  const relationshipCount = graphData?.metadata?.edge_count || 0;
  const recentDocuments = documentsData?.items || [];

  // Calculate unique entity types
  const entityTypes = new Set(graphData?.nodes?.map(n => n.node_type) || []).size;

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
            isLoading={isLoadingDocs}
            variant="documents"
          />
          <StatsCard
            title={t('dashboard.stats.entities', 'Entities')}
            value={entityCount}
            description={t('dashboard.stats.entitiesDesc', 'Extracted entities')}
            icon={Users}
            isLoading={isLoadingGraph}
            variant="entities"
          />
          <StatsCard
            title={t('dashboard.stats.relationships', 'Relationships')}
            value={relationshipCount}
            description={t('dashboard.stats.relationshipsDesc', 'Entity connections')}
            icon={GitMerge}
            isLoading={isLoadingGraph}
            variant="relationships"
          />
          <StatsCard
            title={t('dashboard.stats.entityTypes', 'Entity Types')}
            value={entityTypes}
            description={t('dashboard.stats.entityTypesDesc', 'Unique categories')}
            icon={Network}
            isLoading={isLoadingGraph}
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

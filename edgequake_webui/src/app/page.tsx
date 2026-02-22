/**
 * @module HomePage
 * @description Dashboard home page with stats, recent activity, and quick actions.
 *
 * @implements FEAT0900 - Dashboard overview with stats
 * @implements FEAT0901 - Recent activity feed
 * @implements FEAT0902 - Quick action shortcuts
 *
 * @enforces BR0850 - Stats refresh on tenant/workspace change
 */
'use client';

import { QuickActions, RecentActivity, StatsCard, SystemStatus } from '@/components/dashboard';
import { DynamicBreadcrumb } from '@/components/layout/dynamic-breadcrumb';
import { Header } from '@/components/layout/header';
import { Sidebar } from '@/components/layout/sidebar';
import { SkipLink } from '@/components/shared/skip-link';
import { useKeyboardShortcuts } from '@/hooks/use-keyboard-shortcuts';
import { getDocuments, getWorkspaceStats } from '@/lib/api/edgequake';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useQuery } from '@tanstack/react-query';
import { FileText, GitMerge, Network, Users } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export default function Home() {
  const { t } = useTranslation();
  
  // Enable global keyboard shortcuts
  useKeyboardShortcuts();

  // Get tenant context for query keys
  const { selectedTenantId, selectedWorkspaceId } = useTenantStore();
  
  // Check if context is ready for API calls
  const hasContext = !!selectedTenantId && !!selectedWorkspaceId;

  // Fetch document count - only when context is available
  const { data: documentsData, isLoading: isLoadingDocs } = useQuery({
    queryKey: ['documents', selectedTenantId, selectedWorkspaceId, 1, 10],
    queryFn: () => getDocuments({ page: 1, page_size: 10 }),
    staleTime: 30000,
    enabled: hasContext,
  });

  // WHY: Use getWorkspaceStats() for consistent entity/relationship/entityType counts.
  // Previously the dashboard fetched ALL graph nodes via getGraph({ limit: 1 }) just to
  // compute entityTypes = new Set(nodes.map(n => n.node_type)).size.
  // This was extremely slow for large workspaces (8000+ nodes transferred).
  // Now entity_type_count is computed server-side with a single Cypher aggregate query.
  const { data: statsData, isLoading: isLoadingStats } = useQuery({
    queryKey: ['workspace-stats', selectedTenantId, selectedWorkspaceId],
    queryFn: () => getWorkspaceStats(selectedWorkspaceId!),
    staleTime: 30000,
    enabled: hasContext,
  });

  const documentCount = statsData?.document_count ?? documentsData?.total ?? documentsData?.items?.length ?? 0;
  const entityCount = statsData?.entity_count ?? 0;
  const relationshipCount = statsData?.relationship_count ?? 0;
  const recentDocuments = documentsData?.items || [];
  // WHY: entity_type_count is returned by the backend workspace stats endpoint
  // and counts distinct graph node types (PERSON, ORGANIZATION, etc.) via a
  // single Cypher aggregate query — much faster than fetching all nodes.
  const entityTypes = statsData?.entity_type_count ?? 0;

  return (
    <div className="flex h-screen overflow-hidden bg-background">
      <SkipLink />
      <Sidebar />
      <div className="flex flex-1 flex-col overflow-hidden">
        <Header />
        <div className="border-b px-6 py-2 bg-muted/30">
          <DynamicBreadcrumb />
        </div>
        <main id="main-content" className="flex-1 overflow-auto" tabIndex={-1}>
          <div className="p-6 space-y-6">
            {/* Header */}
            <div>
              <h1 className="text-2xl font-bold">
                {t('dashboard.title', 'Dashboard')}
              </h1>
              <p className="text-muted-foreground">
                {t('dashboard.welcome', 'Welcome to EdgeQuake - Your Knowledge Graph RAG Platform')}
              </p>
            </div>

            {/* Stats Cards */}
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
              <StatsCard
                title={t('dashboard.stats.documents', 'Documents')}
                value={documentCount}
                description={t('dashboard.stats.documentsDesc', 'Uploaded documents')}
                icon={FileText}
                isLoading={isLoadingStats || isLoadingDocs}
              />
              <StatsCard
                title={t('dashboard.stats.entities', 'Entities')}
                value={entityCount}
                description={t('dashboard.stats.entitiesDesc', 'Extracted entities')}
                icon={Users}
                isLoading={isLoadingStats}
              />
              <StatsCard
                title={t('dashboard.stats.relationships', 'Relationships')}
                value={relationshipCount}
                description={t('dashboard.stats.relationshipsDesc', 'Entity connections')}
                icon={GitMerge}
                isLoading={isLoadingStats}
              />
              <StatsCard
                title={t('dashboard.stats.entityTypes', 'Entity Types')}
                value={entityTypes}
                description={t('dashboard.stats.entityTypesDesc', 'Unique categories')}
                icon={Network}
                isLoading={isLoadingStats}
              />
            </div>

            {/* Quick Actions */}
            <QuickActions />

            {/* Recent Activity and System Status */}
            <div className="grid gap-6 lg:grid-cols-3">
              <div className="lg:col-span-2">
                <RecentActivity 
                  documents={recentDocuments} 
                  isLoading={isLoadingDocs}
                />
              </div>
              <div>
                <SystemStatus />
              </div>
            </div>
          </div>
        </main>
      </div>
    </div>
  );
}

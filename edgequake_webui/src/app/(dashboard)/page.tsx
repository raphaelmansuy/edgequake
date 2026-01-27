'use client';

/**
 * @fileoverview Dashboard page with workspace statistics display
 *
 * @implements FEAT1001 - Dashboard statistics visualization
 * @implements FEAT1002 - Workspace-scoped statistics display
 *
 * @see UC1101 - User views knowledge base statistics
 * @see UC1102 - User monitors entity/document counts per workspace
 *
 * @enforces BR1001 - Stats must reflect selected workspace data
 * @enforces BR1002 - Stats update when workspace changes
 */

import { QuickActions, RecentActivity, StatsCard, SystemStatus } from '@/components/dashboard';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useWorkspaceTenantValidator } from '@/hooks/use-workspace-tenant-validator';
import { getDocuments, getWorkspaceStats } from '@/lib/api/edgequake';
import { validateAndClearCache } from '@/lib/cache-manager';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { FileText, GitBranch, Tags, Users } from 'lucide-react';
import { useRouter, useSearchParams } from 'next/navigation';
import { Suspense, useEffect, useRef } from 'react';
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
  const { selectedTenantId, selectedWorkspaceId, _hasHydrated } = useTenantStore();
  
  // Get query client for cache management
  const queryClient = useQueryClient();
  
  // Track if cache has been validated
  const hasValidatedCache = useRef(false);

  // WHY: Validate and clear cache on mount if stale
  // This ensures fresh data is fetched when workspace/tenant changes
  // or after code updates (version change)
  useEffect(() => {
    if (!_hasHydrated) return; // Wait for Zustand to hydrate
    if (hasValidatedCache.current) return; // Only validate once

    hasValidatedCache.current = true;

    // Validate cache and clear if stale
    validateAndClearCache(queryClient, selectedTenantId, selectedWorkspaceId);
  }, [_hasHydrated, selectedTenantId, selectedWorkspaceId, queryClient]);

  // WHY: Force refetch stats when workspace changes
  // This ensures UI always shows current workspace data
  useEffect(() => {
    if (!_hasHydrated || !selectedWorkspaceId) return;

    // Invalidate and refetch stats for the new workspace
    queryClient.invalidateQueries({
      queryKey: ['workspaceStats', selectedWorkspaceId],
    });
    queryClient.refetchQueries({
      queryKey: ['workspaceStats', selectedWorkspaceId],
    });
  }, [selectedWorkspaceId, _hasHydrated, queryClient]);

  // Auto-validate workspace-tenant consistency and fix mismatches
  useWorkspaceTenantValidator({
    onValidationFailed: (result) => {
      console.error('[Dashboard] Workspace-tenant mismatch detected:', result.reason);
    },
  });

  // NOTE: Auto-select logic removed - handled by WorkspaceUrlUpdater component
  // to avoid duplicate selection logic and race conditions

  // WHY: Fetch workspace statistics for the selected workspace
  // This enables the dashboard to show accurate counts that update when workspace changes
  // @implements BR1001 - Stats must reflect selected workspace data
  // OODA-ITERATION-03-CACHE-FIX: Reduced staleTime from 30s to 0 to force fresh fetches
  // This ensures stats are always current, especially after document uploads
  // OODA-ITERATION-04-HYDRATION-FIX: Query now waits for Zustand hydration
  // This prevents racing condition where query runs before workspace ID is loaded from localStorage
  const { data: stats, isLoading: isLoadingStats, error: statsError, isError: isStatsError } = useQuery({
    queryKey: ['workspaceStats', selectedWorkspaceId],
    queryFn: async () => {
      if (!selectedWorkspaceId) {
        throw new Error('No workspace selected');
      }
      
      return await getWorkspaceStats(selectedWorkspaceId);
    },
    enabled: _hasHydrated && !!selectedWorkspaceId, // Wait for hydration!
    staleTime: 0, // Always fetch fresh stats to reflect latest document processing
    refetchOnMount: 'always', // Always refetch when component mounts
  });



  // Fetch recent documents for activity feed
  const { data: documentsData, isLoading: isLoadingDocs } = useQuery({
    queryKey: ['documents', selectedTenantId, selectedWorkspaceId, 1, 10],
    queryFn: () => getDocuments({ page: 1, page_size: 10 }),
    enabled: _hasHydrated && !!selectedWorkspaceId, // Wait for hydration
    staleTime: 30000,
  });

  const recentDocuments = documentsData?.items || [];

  const documentValue = stats?.document_count ?? 0;
  const entityValue = stats?.entity_count ?? 0;
  const relationshipValue = stats?.relationship_count ?? 0;
  const chunkValue = stats?.chunk_count ?? 0;

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

        {/* Statistics Section - Shows workspace-specific counts */}
        {/* @implements FEAT1001 - Dashboard statistics visualization */}
        <section aria-label="Statistics" className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          <StatsCard
            title={t('dashboard.stats.documents', 'Documents')}
            value={documentValue}
            description={t('dashboard.stats.documentsDesc', 'Uploaded documents')}
            icon={FileText}
            variant="documents"
            isLoading={isLoadingStats || !selectedWorkspaceId}
          />
          <StatsCard
            title={t('dashboard.stats.entities', 'Entities')}
            value={entityValue}
            description={t('dashboard.stats.entitiesDesc', 'Extracted entities')}
            icon={Users}
            variant="entities"
            isLoading={isLoadingStats || !selectedWorkspaceId}
          />
          <StatsCard
            title={t('dashboard.stats.relationships', 'Relationships')}
            value={relationshipValue}
            description={t('dashboard.stats.relationshipsDesc', 'Entity connections')}
            icon={GitBranch}
            variant="relationships"
            isLoading={isLoadingStats || !selectedWorkspaceId}
          />
          <StatsCard
            title={t('dashboard.stats.chunks', 'Chunks')}
            value={chunkValue}
            description={t('dashboard.stats.chunksDesc', 'Text segments')}
            icon={Tags}
            variant="types"
            isLoading={isLoadingStats || !selectedWorkspaceId}
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

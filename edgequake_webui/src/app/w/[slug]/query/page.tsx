'use client';

import { useParams } from 'next/navigation';
import { useEffect } from 'react';

import { QueryInterface } from '@/components/query/query-interface';
import { getTenants, getWorkspaceBySlug, getWorkspaces } from '@/lib/api/edgequake';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useQuery } from '@tanstack/react-query';
import { Loader2 } from 'lucide-react';

/**
 * Workspace-scoped query page accessible via deeplink.
 * 
 * @implements SPEC-032: Focus 6 - Deeplinks to workspace
 * @route /w/[slug]/query
 * @iteration OODA 61 - Removed TenantGuard wrapper to fix race condition
 * 
 * This page:
 * 1. Auto-selects tenant if none selected
 * 2. Resolves workspace by slug
 * 3. Sets it as the current workspace in context
 * 4. Renders the query interface directly (no TenantGuard)
 * 
 * Note: TenantGuard was removed because it races with workspace resolution,
 * causing "Create Workspace" UI to appear even when workspaces exist.
 * The deeplink page handles its own loading/error states.
 */
export default function WorkspaceQueryPage() {
  const params = useParams();
  const slug = params?.slug as string;
  
  const { selectedTenantId, selectTenant, selectWorkspace, selectedWorkspaceId, setWorkspaces } = useTenantStore();

  // Fetch tenants to auto-select if needed
  const { data: tenants } = useQuery({
    queryKey: ['tenants'],
    queryFn: getTenants,
    staleTime: 5 * 60 * 1000,
  });

  // Auto-select tenant if only one exists or none selected
  useEffect(() => {
    if (!selectedTenantId && tenants && tenants.length > 0) {
      selectTenant(tenants[0].id);
    }
  }, [selectedTenantId, tenants, selectTenant]);

  // Fetch workspace by slug
  const { data: workspace, isLoading, error } = useQuery({
    queryKey: ['workspace', 'by-slug', selectedTenantId, slug],
    queryFn: () => selectedTenantId ? getWorkspaceBySlug(selectedTenantId, slug) : Promise.reject('No tenant'),
    enabled: !!selectedTenantId && !!slug,
  });

  // Fetch all workspaces to populate store (prevents TenantGuard "no workspaces" race)
  const { data: workspacesData } = useQuery({
    queryKey: ['workspaces', selectedTenantId],
    queryFn: () => selectedTenantId ? getWorkspaces(selectedTenantId) : Promise.resolve([]),
    enabled: !!selectedTenantId,
    staleTime: 5 * 60 * 1000,
  });

  // Update workspace list in store when fetched
  useEffect(() => {
    if (workspacesData && workspacesData.length > 0) {
      setWorkspaces(workspacesData);
    }
  }, [workspacesData, setWorkspaces]);

  // Set workspace context when resolved
  useEffect(() => {
    if (workspace && workspace.id !== selectedWorkspaceId) {
      selectWorkspace(workspace.id);
    }
  }, [workspace, selectedWorkspaceId, selectWorkspace]);

  // Loading state - also wait for tenant selection
  if (isLoading || (!selectedTenantId && tenants === undefined)) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <Loader2 className="h-8 w-8 animate-spin mx-auto text-muted-foreground mb-3" />
          <p className="text-sm text-muted-foreground">Loading workspace...</p>
        </div>
      </div>
    );
  }

  // Error state (404)
  if (error || !workspace) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <h1 className="text-2xl font-semibold mb-2">Workspace Not Found</h1>
          <p className="text-muted-foreground mb-4">
            The workspace &quot;{slug}&quot; does not exist or you don&apos;t have access.
          </p>
          <a href="/workspace" className="text-primary hover:underline">
            Go to Workspace Settings
          </a>
        </div>
      </div>
    );
  }

  // Render query interface directly (no TenantGuard wrapper)
  // TenantGuard was removed in OODA 61 to fix race condition
  return <QueryInterface />;
}

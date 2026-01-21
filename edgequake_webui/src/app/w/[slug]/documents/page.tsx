'use client';

import { getTenants, getWorkspaceBySlug, getWorkspaces } from '@/lib/api/edgequake';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useQuery } from '@tanstack/react-query';
import { Loader2 } from 'lucide-react';
import { useParams, useRouter } from 'next/navigation';
import { useEffect } from 'react';

/**
 * Workspace documents deeplink - sets workspace context and redirects to documents page.
 * 
 * @implements SPEC-032: Focus 6 - Deeplinks to workspace documents
 * @route /w/[slug]/documents
 * @iteration OODA 169 - Added documents deeplink
 */
export default function WorkspaceDocumentsPage() {
  const params = useParams();
  const router = useRouter();
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

  // Fetch all workspaces to populate store
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

  // Set workspace context when resolved and redirect to documents
  useEffect(() => {
    if (workspace) {
      if (workspace.id !== selectedWorkspaceId) {
        selectWorkspace(workspace.id);
      }
      // Redirect to main documents page
      router.push('/documents');
    }
  }, [workspace, selectedWorkspaceId, selectWorkspace, router]);

  // Loading state
  if (isLoading || (!selectedTenantId && tenants === undefined)) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <Loader2 className="h-8 w-8 animate-spin mx-auto text-muted-foreground mb-3" />
          <p className="text-sm text-muted-foreground">Loading workspace documents...</p>
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
          <a href="/documents" className="text-primary hover:underline">
            Go to Documents
          </a>
        </div>
      </div>
    );
  }

  // Will redirect via useEffect
  return (
    <div className="flex items-center justify-center h-full">
      <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
    </div>
  );
}

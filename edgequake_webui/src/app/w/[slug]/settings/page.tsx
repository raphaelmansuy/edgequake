'use client';

import { useParams } from 'next/navigation';
import { useEffect } from 'react';

import { getWorkspaceBySlug } from '@/lib/api/edgequake';
import { useTenantStore } from '@/stores/use-tenant-store';
import { TenantGuard } from '@/components/layout/tenant-guard';
import { Loader2 } from 'lucide-react';
import { useQuery } from '@tanstack/react-query';
import { useRouter } from 'next/navigation';

/**
 * Workspace settings page accessible via deeplink.
 * 
 * @implements SPEC-032: Focus 6 - Deeplinks to workspace settings
 * @route /w/[slug]/settings
 * 
 * This page:
 * 1. Resolves workspace by slug
 * 2. Sets it as the current workspace in context
 * 3. Redirects to /workspace (which shows settings for current workspace)
 */
export default function WorkspaceSettingsPage() {
  const params = useParams();
  const router = useRouter();
  const slug = params?.slug as string;
  
  const { selectedTenantId, selectWorkspace, selectedWorkspaceId } = useTenantStore();

  // Fetch workspace by slug
  const { data: workspace, isLoading, error } = useQuery({
    queryKey: ['workspace', 'by-slug', selectedTenantId, slug],
    queryFn: () => selectedTenantId ? getWorkspaceBySlug(selectedTenantId, slug) : Promise.reject('No tenant'),
    enabled: !!selectedTenantId && !!slug,
  });

  // Set workspace context when resolved and redirect to settings
  useEffect(() => {
    if (workspace) {
      if (workspace.id !== selectedWorkspaceId) {
        selectWorkspace(workspace.id);
      }
      // Redirect to main workspace settings page
      router.push('/workspace');
    }
  }, [workspace, selectedWorkspaceId, selectWorkspace, router]);

  // Loading state
  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <Loader2 className="h-8 w-8 animate-spin mx-auto text-muted-foreground mb-3" />
          <p className="text-sm text-muted-foreground">Loading workspace settings...</p>
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

  // Will redirect via useEffect
  return (
    <TenantGuard>
      <div className="flex items-center justify-center h-full">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    </TenantGuard>
  );
}

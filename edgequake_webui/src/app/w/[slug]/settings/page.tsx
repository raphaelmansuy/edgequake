'use client';

import { useParams, useRouter } from 'next/navigation';
import { useEffect } from 'react';

import {
    WorkspaceLoading,
    WorkspaceNotFound,
    WorkspaceRedirecting,
} from '@/components/workspace/workspace-deeplink-states';
import { useWorkspaceSlugResolver } from '@/hooks/use-workspace-slug-resolver';

/**
 * Workspace settings page accessible via deeplink.
 *
 * @implements SPEC-032: Focus 6 - Deeplinks to workspace settings
 * @route /w/[slug]/settings
 *
 * WHY: Uses useWorkspaceSlugResolver (DRY) for workspace resolution.
 * SRP: This page only handles routing → redirect to /workspace.
 */
export default function WorkspaceSettingsPage() {
  const params = useParams();
  const router = useRouter();
  const slug = params?.slug as string;

  const { workspace, isLoading, error, isReady } = useWorkspaceSlugResolver(slug);

  // Redirect once workspace is resolved and synced
  useEffect(() => {
    if (isReady) {
      router.push('/workspace');
    }
  }, [isReady, router]);

  if (isLoading) {
    return <WorkspaceLoading context="workspace settings" />;
  }

  if (error || !workspace) {
    return (
      <WorkspaceNotFound
        slug={slug}
        fallbackHref="/workspace"
        fallbackLabel="Go to Workspace Settings"
      />
    );
  }

  // Will redirect via useEffect
  return <WorkspaceRedirecting />;
}

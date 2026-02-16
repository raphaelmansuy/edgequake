'use client';

import {
    WorkspaceLoading,
    WorkspaceNotFound,
    WorkspaceRedirecting,
} from '@/components/workspace/workspace-deeplink-states';
import { useWorkspaceSlugResolver } from '@/hooks/use-workspace-slug-resolver';
import { useParams, useRouter } from 'next/navigation';
import { useEffect } from 'react';

/**
 * Workspace graph deeplink - sets workspace context and redirects to graph page.
 *
 * @implements SPEC-032: Focus 6 - Deeplinks to workspace graph
 * @route /w/[slug]/graph
 *
 * WHY: Uses useWorkspaceSlugResolver (DRY) for workspace resolution.
 * SRP: This page only handles routing → redirect to /graph.
 */
export default function WorkspaceGraphPage() {
  const params = useParams();
  const router = useRouter();
  const slug = params?.slug as string;

  const { workspace, isLoading, error, isReady } = useWorkspaceSlugResolver(slug);

  // Redirect once workspace is resolved and synced
  useEffect(() => {
    if (isReady) {
      router.push('/graph');
    }
  }, [isReady, router]);

  if (isLoading) {
    return <WorkspaceLoading context="workspace graph" />;
  }

  if (error || !workspace) {
    return (
      <WorkspaceNotFound
        slug={slug}
        fallbackHref="/graph"
        fallbackLabel="Go to Graph"
      />
    );
  }

  // Will redirect via useEffect
  return <WorkspaceRedirecting />;
}

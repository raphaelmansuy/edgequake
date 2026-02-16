'use client';

import { useParams } from 'next/navigation';

import { QueryInterface } from '@/components/query/query-interface';
import {
    WorkspaceLoading,
    WorkspaceNotFound,
} from '@/components/workspace/workspace-deeplink-states';
import { useWorkspaceSlugResolver } from '@/hooks/use-workspace-slug-resolver';

/**
 * Workspace-scoped query page accessible via deeplink.
 *
 * @implements SPEC-032: Focus 6 - Deeplinks to workspace
 * @route /w/[slug]/query
 *
 * WHY: Uses useWorkspaceSlugResolver (DRY) instead of inline tenant/workspace
 * resolution. The hook handles tenant auto-select, slug resolution, and store sync.
 * This page's single responsibility is rendering the QueryInterface once ready.
 */
export default function WorkspaceQueryPage() {
  const params = useParams();
  const slug = params?.slug as string;

  const { workspace, isLoading, error } = useWorkspaceSlugResolver(slug);

  if (isLoading) {
    return <WorkspaceLoading context="workspace" />;
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

  return <QueryInterface />;
}

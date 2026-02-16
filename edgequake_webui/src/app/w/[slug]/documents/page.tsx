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
 * Workspace documents deeplink - sets workspace context and redirects to documents page.
 *
 * @implements SPEC-032: Focus 6 - Deeplinks to workspace documents
 * @route /w/[slug]/documents
 *
 * WHY: Uses useWorkspaceSlugResolver (DRY) for workspace resolution.
 * SRP: This page only handles routing → redirect to /documents.
 */
export default function WorkspaceDocumentsPage() {
  const params = useParams();
  const router = useRouter();
  const slug = params?.slug as string;

  const { workspace, isLoading, error, isReady } = useWorkspaceSlugResolver(slug);

  // Redirect once workspace is resolved and synced
  useEffect(() => {
    if (isReady) {
      router.push('/documents');
    }
  }, [isReady, router]);

  if (isLoading) {
    return <WorkspaceLoading context="workspace documents" />;
  }

  if (error || !workspace) {
    return (
      <WorkspaceNotFound
        slug={slug}
        fallbackHref="/documents"
        fallbackLabel="Go to Documents"
      />
    );
  }

  // Will redirect via useEffect
  return <WorkspaceRedirecting />;
}

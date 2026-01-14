'use client';

import { useRouter } from 'next/navigation';
import { use, useEffect } from 'react';

/**
 * Workspace deeplink - redirects to query page.
 * 
 * @implements SPEC-032: Focus 6 - Deeplinks to workspace
 * @route /w/[slug]
 */
export default function WorkspaceSlugPage({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const router = useRouter();
  const { slug } = use(params);

  useEffect(() => {
    // Redirect to query page
    router.replace(`/w/${slug}/query`);
  }, [router, slug]);

  return (
    <div className="flex items-center justify-center h-screen">
      <div className="text-muted-foreground">Loading workspace...</div>
    </div>
  );
}

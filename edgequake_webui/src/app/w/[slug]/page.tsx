'use client';

import { redirect } from 'next/navigation';

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
  // Next.js 15 requires async params
  // Redirect to query page
  return redirect(`/w/${(params as unknown as { slug: string }).slug}/query`);
}

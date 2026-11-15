'use client';

import { capabilitiesOf, type RagProfileCapabilities } from '@/constants/rag-profile';
import { useSelectedWorkspace } from '@/stores/use-tenant-store';

/**
 * SPEC-180 — what the selected workspace can do, as the server stated it.
 * Components read these booleans; none compares the profile name.
 *
 * WHY the unknown case says "graph": an older workspace without the field really
 * is a graph one, so that fallback is the honest answer for a *loaded* payload.
 * It is the wrong answer for "nothing has loaded yet" — see
 * `useWorkspaceCapabilitiesOrNull`, which the empty Query screen uses so a Normal
 * workspace never flashes graph-shaped suggestions while it is still fetching.
 */
export function useWorkspaceCapabilities(): RagProfileCapabilities {
  return capabilitiesOf(useSelectedWorkspace());
}

/**
 * The same capabilities, but `null` until a workspace payload exists.
 *
 * WHY this exists: the empty Query screen derives its suggestions from these
 * booleans. Defaulting to graph made a Normal workspace render four knowledge-graph
 * questions for the first frames — visible on a slow load and a CI flake for the
 * SPEC-184 image corpus test. `null` lets the caller render nothing until it knows.
 */
export function useWorkspaceCapabilitiesOrNull(): RagProfileCapabilities | null {
  const workspace = useSelectedWorkspace();
  return workspace?.rag_profile_capabilities ?? null;
}

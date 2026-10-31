'use client';

import { capabilitiesOf, type RagProfileCapabilities } from '@/constants/rag-profile';
import { useSelectedWorkspace } from '@/stores/use-tenant-store';

/**
 * SPEC-180 — what the selected workspace can do, as the server stated it.
 * Components read these booleans; none compares the profile name.
 * Until a workspace loads, the answer is the graph one so nothing flickers away.
 */
export function useWorkspaceCapabilities(): RagProfileCapabilities {
  return capabilitiesOf(useSelectedWorkspace());
}

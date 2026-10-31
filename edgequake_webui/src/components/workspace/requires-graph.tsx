'use client';

import { NoGraphState } from '@/components/workspace/no-graph-state';
import { useWorkspaceCapabilities } from '@/hooks/use-workspace-capabilities';

/** SPEC-180 — render a graph page only on a workspace that has a graph; explain otherwise. */
export function RequiresGraph({ children }: { children: React.ReactNode }) {
  const { shows_graph_ui } = useWorkspaceCapabilities();
  return shows_graph_ui ? <>{children}</> : <NoGraphState />;
}

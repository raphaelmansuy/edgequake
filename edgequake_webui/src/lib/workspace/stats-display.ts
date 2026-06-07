import type { WorkspaceStats } from "@/lib/api/edgequake/workspaces";
import type { Workspace } from "@/types";

/** Resolved counts for workspace stats cards (SPEC-017 UI-P3-001). */
export interface WorkspaceStatCounts {
  documents: number;
  entities: number;
  relationships: number;
  chunks: number;
}

export function resolveWorkspaceStatCounts(
  stats: WorkspaceStats | undefined,
  workspace: Workspace,
): WorkspaceStatCounts {
  return {
    documents: stats?.document_count ?? workspace.document_count ?? 0,
    entities: stats?.entity_count ?? workspace.entity_count ?? 0,
    relationships: stats?.relationship_count ?? 0,
    chunks: stats?.chunk_count ?? 0,
  };
}

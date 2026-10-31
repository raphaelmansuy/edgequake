/**
 * SPEC-180 — document-kind chips of the query composer.
 *
 * Kept per workspace and in memory only, so a graph workspace never inherits an image filter
 * from a normal one, and a reload starts unfiltered ("empty means all").
 */
import { create } from 'zustand';
import type { DocKind } from '@/constants/rag-profile';

interface QueryKindsState {
  byWorkspace: Record<string, readonly DocKind[]>;
  toggle: (workspaceId: string, kind: DocKind) => void;
  clear: (workspaceId: string) => void;
}

const NONE: readonly DocKind[] = [];

export const useQueryKindsStore = create<QueryKindsState>((set) => ({
  byWorkspace: {},
  toggle: (workspaceId, kind) =>
    set((state) => {
      const current = state.byWorkspace[workspaceId] ?? NONE;
      const next = current.includes(kind) ? current.filter((k) => k !== kind) : [...current, kind];
      return { byWorkspace: { ...state.byWorkspace, [workspaceId]: next } };
    }),
  clear: (workspaceId) =>
    set((state) => ({ byWorkspace: { ...state.byWorkspace, [workspaceId]: NONE } })),
}));

/** The chips selected for a workspace; a stable empty array when there are none. */
export function selectKinds(workspaceId: string | null) {
  return (state: QueryKindsState): readonly DocKind[] =>
    (workspaceId ? state.byWorkspace[workspaceId] : undefined) ?? NONE;
}

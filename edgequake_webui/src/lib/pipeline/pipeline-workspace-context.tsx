"use client";

import { createContext, useContext } from "react";

export interface PipelineWorkspaceContextValue {
  selectedTenantId: string | null;
  selectedWorkspaceId: string | null;
  workspaceName: string;
}

export const PipelineWorkspaceContext =
  createContext<PipelineWorkspaceContextValue>({
    selectedTenantId: null,
    selectedWorkspaceId: null,
    workspaceName: "All Workspaces",
  });

export function usePipelineWorkspace(): PipelineWorkspaceContextValue {
  return useContext(PipelineWorkspaceContext);
}

/** Scoped query key including tenant/workspace for cache isolation (OODA-37). */
export function scopedQueryKey(
  base: string,
  tenantId: string | null,
  workspaceId: string | null,
): (string | null)[] {
  return [base, tenantId, workspaceId];
}

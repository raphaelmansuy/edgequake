"use client";

/**
 * SPEC-175 W4/W5 — server state for the typed model editors.
 *
 * One query key object for every model-policy read, so writes invalidate exactly what they
 * change. Mutations write the returned policy into the cache and refresh the effective usages
 * the resolution ladder depends on.
 */

import {
  fetchEffectiveUsages,
  fetchEligibleModels,
  fetchEmbeddingSupport,
  fetchScopedEligibleModels,
  fetchTenantModelCatalog,
  getTenantModelPolicy,
  getWorkspaceModelPolicy,
  putTenantModelPolicy,
  putWorkspaceModelPolicy,
  type EligibleModel,
  type PolicyRow,
  type TenantPolicyResponse,
  type UsageKey,
  type WorkspacePolicyResponse,
  USAGE_KEYS,
} from "@/lib/api/edgequake/model-catalog";
import { apiClient } from "@/lib/api/client";
import { selectableConnections } from "@/lib/settings/connection-options";
import { useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";

export const modelPolicyQueryKeys = {
  effectiveRoot: ["effective-usages"] as const,
  effective: (workspaceId: string) => ["effective-usages", workspaceId] as const,
  eligibleRoot: ["eligible-models"] as const,
  eligible: (workspaceId: string, usage: UsageKey) =>
    ["eligible-models", "workspace", workspaceId, usage] as const,
  eligibleScoped: (scope: string, id: string, usage: UsageKey) =>
    ["eligible-models", scope, id, usage] as const,
  catalog: (tenantId: string) => ["tenant-model-catalog", tenantId] as const,
  tenantPolicy: (tenantId: string) => ["tenant-model-policy", tenantId] as const,
  workspacePolicy: (workspaceId: string) => ["workspace-model-policy", workspaceId] as const,
  connections: ["connections"] as const,
};

/** Connections the workspace role editor can pick. A failed load yields an empty list. */
export interface ConnectionOption {
  id: string;
  slug: string;
  display_name: string;
  source: string;
}

export function useConnectionOptions() {
  return useQuery({
    queryKey: modelPolicyQueryKeys.connections,
    queryFn: async () => {
      const data = await apiClient<ConnectionOption[]>("/connections");
      return Array.isArray(data) ? data : [];
    },
    // `select` runs on the shared cache. The connections screen stores the full list under the same key.
    select: (data) => selectableConnections(data),
    staleTime: 60_000,
    retry: false,
  });
}

export function useEffectiveUsages(workspaceId: string | null) {
  return useQuery({
    queryKey: modelPolicyQueryKeys.effective(workspaceId ?? ""),
    queryFn: () => fetchEffectiveUsages(workspaceId as string),
    enabled: !!workspaceId,
    staleTime: 15_000,
    retry: false,
  });
}

export function useEligibleModels(workspaceId: string | null, usage: UsageKey) {
  return useScopedEligibleModels({ scope: "workspace", id: workspaceId, usage });
}

export function useScopedEligibleModels(input: {
  scope: "workspace" | "tenant" | "deployment";
  id?: string | null;
  usage: UsageKey;
}) {
  const id = input.scope === "deployment" ? "deployment" : (input.id ?? "");
  const enabled = input.scope === "deployment" || Boolean(input.id);
  return useQuery({
    queryKey: modelPolicyQueryKeys.eligibleScoped(input.scope, id, input.usage),
    queryFn: () => fetchScopedEligibleModels({ scope: input.scope, id: input.id, usage: input.usage }),
    enabled,
    staleTime: 15_000,
    retry: false,
  });
}

const CHAT_FAMILY = ["extract", "query", "summary", "keyword"] as const;

/**
 * SPEC-180 08 — the model step's own answer to "can this deployment embed images with the
 * pair I am about to save?". Create enforces the same probe, so the inline reason and the
 * refusal cannot disagree. Disabled until a pair is complete and images are wanted.
 */
export function useEmbeddingSupport(input: {
  scope: "workspace" | "tenant" | "deployment";
  id?: string | null;
  provider?: string;
  model?: string;
  dimension?: number;
  enabled?: boolean;
}) {
  const ready = Boolean(input.provider && input.model) && input.enabled !== false;
  return useQuery({
    queryKey: [
      "embedding-support",
      input.scope,
      input.scope === "deployment" ? "deployment" : (input.id ?? ""),
      input.provider ?? "",
      input.model ?? "",
      input.dimension ?? 0,
    ],
    queryFn: () =>
      fetchEmbeddingSupport({
        scope: input.scope,
        id: input.id,
        provider: input.provider as string,
        model: input.model as string,
        dimension: input.dimension,
      }),
    enabled: ready,
    // The server caches the probe for five minutes; do not re-ask on every focus.
    staleTime: 60_000,
    retry: false,
  });
}
/** Intersection of the four chat usages. A view; the save still judges each usage. */
export function useChatFamilyModels(input: {
  scope: "workspace" | "tenant" | "deployment";
  id?: string | null;
}) {
  const results = useQueries({
    queries: CHAT_FAMILY.map((usage) => ({
      queryKey: modelPolicyQueryKeys.eligibleScoped(
        input.scope,
        input.scope === "deployment" ? "deployment" : (input.id ?? ""),
        usage,
      ),
      queryFn: () => fetchScopedEligibleModels({ ...input, usage }),
      enabled: input.scope === "deployment" || Boolean(input.id),
      staleTime: 15_000,
      retry: false,
    })),
  });
  const lists = results.map((result) => result.data?.models ?? []);
  const [first, ...rest] = lists;
  const models = (first ?? []).filter((model) =>
    rest.every((list) =>
      list.some((other) => other.provider === model.provider && other.model === model.model),
    ),
  );
  return {
    models,
    isLoading: results.some((result) => result.isPending && result.fetchStatus !== "idle"),
    isError: results.some((result) => result.isError),
  };
}

/** Eligible models for every usage, keyed by usage. Used by the narrowing editor. */
export function useEligibleModelsByUsage(workspaceId: string | null) {
  const results = useQueries({
    queries: USAGE_KEYS.map((usage) => ({
      queryKey: modelPolicyQueryKeys.eligible(workspaceId ?? "", usage),
      queryFn: () => fetchEligibleModels(workspaceId as string, usage),
      enabled: !!workspaceId,
      staleTime: 15_000,
      retry: false,
    })),
  });
  const byUsage = {} as Record<UsageKey, EligibleModel[]>;
  USAGE_KEYS.forEach((usage, i) => {
    byUsage[usage] = results[i]?.data?.models ?? [];
  });
  return { byUsage, loading: results.some((r) => r.isPending && r.fetchStatus !== "idle") };
}

export function useTenantModelCatalog(tenantId: string | null) {
  return useQuery({
    queryKey: modelPolicyQueryKeys.catalog(tenantId ?? ""),
    queryFn: () => fetchTenantModelCatalog(tenantId as string),
    enabled: !!tenantId,
    staleTime: 30_000,
    retry: false,
  });
}

export function useTenantModelPolicy(tenantId: string | null) {
  return useQuery({
    queryKey: modelPolicyQueryKeys.tenantPolicy(tenantId ?? ""),
    queryFn: () => getTenantModelPolicy(tenantId as string),
    enabled: !!tenantId,
    staleTime: 15_000,
    retry: false,
  });
}

export function useWorkspaceModelPolicy(workspaceId: string | null) {
  return useQuery({
    queryKey: modelPolicyQueryKeys.workspacePolicy(workspaceId ?? ""),
    queryFn: () => getWorkspaceModelPolicy(workspaceId as string),
    enabled: !!workspaceId,
    staleTime: 15_000,
    retry: false,
  });
}

/** Full-replace save of the tenant ceiling. The server returns the new policy. */
export function useSaveTenantModelPolicy(tenantId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (rows: PolicyRow[]) => putTenantModelPolicy(tenantId, rows),
    onSuccess: (policy: TenantPolicyResponse) => {
      queryClient.setQueryData(modelPolicyQueryKeys.tenantPolicy(tenantId), policy);
      // A ceiling change moves every workspace's eligible models and resolved usages.
      void queryClient.invalidateQueries({ queryKey: modelPolicyQueryKeys.effectiveRoot });
      void queryClient.invalidateQueries({ queryKey: modelPolicyQueryKeys.eligibleRoot });
    },
  });
}

/** Full-replace save of one workspace's narrowing rows. */
export function useSaveWorkspaceModelPolicy(workspaceId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (rows: PolicyRow[]) => putWorkspaceModelPolicy(workspaceId, rows),
    onSuccess: (policy: WorkspacePolicyResponse) => {
      queryClient.setQueryData(modelPolicyQueryKeys.workspacePolicy(workspaceId), policy);
      void queryClient.invalidateQueries({ queryKey: modelPolicyQueryKeys.effective(workspaceId) });
      void queryClient.invalidateQueries({ queryKey: modelPolicyQueryKeys.eligibleRoot });
    },
  });
}

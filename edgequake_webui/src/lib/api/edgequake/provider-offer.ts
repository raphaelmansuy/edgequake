import { ApiRequestError, apiClient } from "@/lib/api/client";

export interface OfferModel {
  id: string;
  usages: string[];
  embedding_dimension?: number;
  /** False for a model the provider reported but nobody has registered yet. */
  registered?: boolean;
}

export interface DeploymentProvider {
  slug: string;
  display_name: string;
  origin: string;
  api_shape: string;
  /** Where this provider is called. Older servers omit it. */
  base_url?: string;
  offered: boolean;
  withdrawn: boolean;
  probe: string;
  key_configured: boolean;
  /**
   * SPEC-184 — how the provider authenticates (`api_key`, `server_google_identity`, …) and whether
   * a stored key means anything. Optional so a payload from an older server still parses; when
   * absent the card falls back to `key_configured`, which is what it always used.
   */
  auth?: string;
  accepts_saved_key?: boolean;
  /** Saved connection this card can edit and test. */
  connection_id?: string | null;
  /** Why the last test failed. Absent until a test has failed. */
  probe_error?: string | null;
  /** Saved Google Cloud project. Present on a Vertex connection that has one. */
  google_project?: string | null;
  /** Human name for `api_shape` ("OpenAI-compatible"), from the shape registry. */
  api_shape_label?: string;
  models: OfferModel[];
  refresh?: {
    refreshed_at: string;
    status: "ok" | "error";
    error?: string | null;
    added: number;
    removed: number;
  };
}

export interface OfferProvider {
  slug: string;
  display_name: string;
  offered_by_parent: boolean;
  enabled: boolean;
  models: OfferModel[];
  /**
   * SPEC-184 — the security contract, so a row can say what the provider needs instead of only how
   * many models it has. Optional: an older server omits them and the row stays as it was.
   */
  auth?: string;
  key_configured?: boolean;
}

export interface ProviderOffer {
  tenant_id: string;
  workspace_id?: string | null;
  offer_mode: "unconstrained" | "constrained";
  parent_count: number;
  enabled_count: number;
  can_manage: boolean;
  providers: OfferProvider[];
  legacy: { slug: string; display_name: string }[];
  revision?: number;
}

export interface PutOfferBody {
  offer_mode: "unconstrained" | "constrained";
  slugs: string[];
}

export const fetchDeploymentProviders = () =>
  apiClient<{ providers: DeploymentProvider[] }>("/deployment/providers");

export const withdrawProvider = (slug: string, withdrawn: boolean, confirm = false) =>
  apiClient<DeploymentProvider>(
    `/deployment/providers/${encodeURIComponent(slug)}${confirm ? "?confirm=true" : ""}`,
    {
      method: "PUT",
      body: JSON.stringify({ withdrawn }),
    },
  );

export interface ProviderRefreshResult {
  status: "ok" | "error";
  error?: string | null;
  added: string[];
  removed: string[];
  refreshed_at: string;
}

export const refreshProviderModels = (slug: string) =>
  apiClient<ProviderRefreshResult>(`/deployment/providers/${encodeURIComponent(slug)}/refresh`, {
    method: "POST",
  });

export interface ProviderImpact {
  assignments: { scope: string; id: string; field: string }[];
}

export const fetchProviderImpact = (slug: string) =>
  apiClient<ProviderImpact>(`/deployment/providers/${encodeURIComponent(slug)}/impact`);

/** True when the server wants an explicit confirm because assignments still use the provider. */
export const needsConfirm = (error: unknown): error is ApiRequestError =>
  error instanceof ApiRequestError && error.status === 409 && error.code === "confirm_required";

export const putConnectionModels = (
  slug: string,
  models: {
    id: string;
    supports_chat: boolean;
    supports_vision: boolean;
    supports_embedding: boolean;
    embedding_dimension?: number;
  }[],
) =>
  apiClient<void>(`/deployment/providers/${encodeURIComponent(slug)}/models`, {
    method: "PUT",
    body: JSON.stringify({ models }),
  });

export const fetchTenantOffer = (tenantId: string) =>
  apiClient<ProviderOffer>(`/tenants/${tenantId}/provider-offer`);

export const putTenantOffer = (tenantId: string, body: PutOfferBody, revision?: number) =>
  apiClient<ProviderOffer>(`/tenants/${tenantId}/provider-offer`, {
    method: "PUT",
    body: JSON.stringify(body),
    headers: revision === undefined ? undefined : { "If-Match": String(revision) },
  });

export const fetchWorkspaceOffer = (workspaceId: string) =>
  apiClient<ProviderOffer>(`/workspaces/${workspaceId}/provider-offer`);

export const putWorkspaceOffer = (workspaceId: string, body: PutOfferBody, revision?: number) =>
  apiClient<ProviderOffer>(`/workspaces/${workspaceId}/provider-offer`, {
    method: "PUT",
    body: JSON.stringify(body),
    headers: revision === undefined ? undefined : { "If-Match": String(revision) },
  });

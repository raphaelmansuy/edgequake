/**
 * SPEC-169 — organization members, invitations and the role catalog.
 *
 * Mirrors `handlers/members/*` and `GET /roles`. Two role axes exist on the wire: the
 * *organization* role (`owner|admin|member|readonly`, per tenant) and the *instance* role
 * (`admin|user|readonly`, platform-wide). The UI labels `readonly` as "Viewer".
 */

import { api } from "../client";

export type OrgRole = "owner" | "admin" | "member" | "readonly";
export type InstanceRole = "admin" | "user" | "readonly";
export type MemberKind = "member" | "invitation";
export type MemberStatus = "active" | "deactivated" | "invited" | "expired";

export interface SignInMethod {
  kind: "password" | "sso";
  provider_slug?: string | null;
  provider_name?: string | null;
  /** `keycloak` | `google` | `microsoft` | … */
  provider_kind?: string | null;
}

export interface MemberView {
  /** User id, or the invitation id for invitations. */
  id: string;
  kind: MemberKind;
  user_id?: string | null;
  username?: string | null;
  email: string;
  role: OrgRole;
  workspace_id?: string | null;
  status: MemberStatus;
  instance_role?: InstanceRole | null;
  sign_in: SignInMethod[];
  /** Provider slug when the role is re-synced from the IdP at every sign-in. */
  role_managed_by?: string | null;
  joined_at: string;
  last_login_at?: string | null;
  invited_by?: string | null;
  expires_at?: string | null;
  is_you: boolean;
  is_last_owner: boolean;
  /** SPEC-181: every active grant; `role` above stays the primary one. */
  grants?: GrantSummary[];
}

export interface MemberSummary {
  total: number;
  active: number;
  deactivated: number;
  invited: number;
  by_role: Partial<Record<OrgRole, number>>;
}

export interface MemberViewer {
  user_id: string;
  role?: OrgRole | null;
  instance_admin: boolean;
  max_assignable_role: OrgRole;
}

export interface MemberListResponse {
  members: MemberView[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
  summary: MemberSummary;
  viewer: MemberViewer;
  seats: { used: number; max?: number | null };
  tenant_name: string;
}

export type MemberSort = "name" | "role" | "last_active" | "joined";

export interface MemberListParams {
  q?: string;
  role?: OrgRole | "all";
  status?: MemberStatus | "all";
  sort?: MemberSort;
  dir?: "asc" | "desc";
  page?: number;
  page_size?: number;
}

export interface UpdateMemberBody {
  role?: OrgRole;
  /** Platform admins only. */
  is_active?: boolean;
  /** Platform admins only. */
  instance_role?: InstanceRole;
}

export interface InvitationIssued {
  invitation: MemberView;
  /** Single-use accept token — shown once; build `/invite/{token}` from it. */
  token: string;
  expires_in_days: number;
}

export interface InvitationPreview {
  tenant_name: string;
  email: string;
  role: OrgRole;
  role_label: string;
  invited_by?: string | null;
  expires_at: string;
  account_exists: boolean;
  sso_providers: { slug: string; display_name: string; kind: string; login_path: string }[];
}

export interface AcceptInvitationResponse {
  tenant_id: string;
  tenant_name: string;
  role: OrgRole;
  email: string;
  username: string;
  created_account: boolean;
}

export interface RoleCatalog {
  organization_roles: { id: OrgRole; label: string; description: string; level: number }[];
  instance_roles: { id: InstanceRole; label: string; description: string; permission_count: number }[];
  capabilities: { id: string; group: string; label: string; roles: OrgRole[] }[];
}

function query(params: MemberListParams): string {
  const qs = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value === undefined || value === "" || value === "all") continue;
    qs.set(key, String(value));
  }
  const s = qs.toString();
  return s ? `?${s}` : "";
}

const base = (tenantId: string) => `/tenants/${encodeURIComponent(tenantId)}`;

export const listMembers = (tenantId: string, params: MemberListParams = {}) =>
  api.get<MemberListResponse>(`${base(tenantId)}/members${query(params)}`);

export const updateMember = (tenantId: string, userId: string, body: UpdateMemberBody) =>
  api.patch<MemberView>(`${base(tenantId)}/members/${encodeURIComponent(userId)}`, body);

export const removeMember = (tenantId: string, userId: string) =>
  api.delete<void>(`${base(tenantId)}/members/${encodeURIComponent(userId)}`);

export const createInvitation = (
  tenantId: string,
  body: { email: string; role: OrgRole; workspace_id?: string },
) => api.post<InvitationIssued>(`${base(tenantId)}/invitations`, body);

export const resendInvitation = (tenantId: string, invitationId: string) =>
  api.post<InvitationIssued>(`${base(tenantId)}/invitations/${invitationId}/resend`);

export const revokeInvitation = (tenantId: string, invitationId: string) =>
  api.delete<void>(`${base(tenantId)}/invitations/${invitationId}`);

export const getRoleCatalog = () => api.get<RoleCatalog>("/roles");

export const previewInvitation = (token: string) =>
  api.get<InvitationPreview>(`/auth/invitations/${encodeURIComponent(token)}`);

export const acceptInvitation = (
  token: string,
  body: { username?: string; password?: string },
) => api.post<AcceptInvitationResponse>(`/auth/invitations/${encodeURIComponent(token)}/accept`, body);

/** Absolute browser URL of an accept link. */
export const inviteUrl = (token: string, origin = window.location.origin) =>
  `${origin}/invite/${token}`;

// ── SPEC-181: workspace-scoped grants ───────────────────────────────────────

export interface GrantView {
  membership_id: string;
  user_id: string;
  scope: "organization" | "workspace";
  workspace_id?: string | null;
  workspace_name?: string | null;
  role: OrgRole;
  level: number;
  is_active: boolean;
  sso_managed: boolean;
  joined_at: string;
}

/** One grant on a directory row (SPEC-181 `MemberView.grants`). */
export interface GrantSummary {
  scope: "organization" | "workspace";
  workspace_id?: string | null;
  workspace_name?: string | null;
  role: OrgRole;
}

export const listGrants = (tenantId: string, params: { user_id?: string; workspace_id?: string } = {}) => {
  const qs = new URLSearchParams();
  if (params.user_id) qs.set("user_id", params.user_id);
  if (params.workspace_id) qs.set("workspace_id", params.workspace_id);
  const q = qs.toString();
  return api.get<GrantView[]>(`${base(tenantId)}/grants${q ? `?${q}` : ""}`);
};

export const createGrant = (
  tenantId: string,
  body: { user_id: string; workspace_id?: string | null; role: OrgRole },
) => api.post<GrantView>(`${base(tenantId)}/grants`, body);

export const updateGrant = (tenantId: string, membershipId: string, role: OrgRole) =>
  api.patch<GrantView>(`${base(tenantId)}/grants/${encodeURIComponent(membershipId)}`, { role });

export const deleteGrant = (tenantId: string, membershipId: string) =>
  api.delete<void>(`${base(tenantId)}/grants/${encodeURIComponent(membershipId)}`);

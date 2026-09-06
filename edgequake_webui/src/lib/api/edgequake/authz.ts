/**
 * SPEC-146 authz PAP client — members/roles/attrs/policies/break-glass.
 */

import { api } from "@/lib/api/client";

export interface WorkspaceRoleDto {
  role_id: string;
  workspace_id: string;
  name: string;
  is_builtin: boolean;
  permissions: string[];
}

export interface RoleBindingDto {
  workspace_id: string;
  principal_kind: string;
  principal_id: string;
  role_id: string;
  role_name?: string;
}

export interface AttributeDefinitionDto {
  attr_id: string;
  workspace_id: string;
  scope: string;
  name: string;
  value_type: string;
  enum_values?: unknown;
  required_for_share_modes: string[];
}

export interface PrincipalAttributeDto {
  workspace_id: string;
  principal_kind: string;
  principal_id: string;
  name: string;
  value: unknown;
  source: string;
  updated_at: string;
}

export interface PolicyDto {
  policy_id: string;
  workspace_id: string;
  name: string;
  active_version: number;
}

export interface BreakGlassSessionDto {
  session_id: string;
  workspace_id: string;
  principal_kind: string;
  principal_id: string;
  reason: string;
  expires_at: string;
  scope_doc_ids?: string[];
  created_at: string;
  revoked_at?: string;
}

function wsBase(workspaceId: string) {
  return `/workspaces/${workspaceId}/authz`;
}

export async function listAuthzRoles(workspaceId: string) {
  return api.get<{ roles: WorkspaceRoleDto[] }>(`${wsBase(workspaceId)}/roles`);
}

export async function createAuthzRole(
  workspaceId: string,
  body: { name: string; permissions?: string[] },
) {
  return api.post<{ role: WorkspaceRoleDto }>(`${wsBase(workspaceId)}/roles`, body);
}

export async function deleteAuthzRole(workspaceId: string, roleId: string) {
  return api.delete(`${wsBase(workspaceId)}/roles/${roleId}`);
}

export async function listAuthzMembers(workspaceId: string) {
  return api.get<{ bindings: RoleBindingDto[] }>(`${wsBase(workspaceId)}/members`);
}

export async function createAuthzMember(
  workspaceId: string,
  body: { principal_kind: string; principal_id: string; role_id: string },
) {
  return api.post<{ binding: RoleBindingDto }>(`${wsBase(workspaceId)}/members`, body);
}

export async function deleteAuthzMember(
  workspaceId: string,
  principalKind: string,
  principalId: string,
  roleId: string,
) {
  return api.delete(
    `${wsBase(workspaceId)}/members/${encodeURIComponent(principalKind)}/${encodeURIComponent(principalId)}/${roleId}`,
  );
}

export async function listAttributeDefinitions(workspaceId: string) {
  return api.get<{ attributes: AttributeDefinitionDto[] }>(
    `${wsBase(workspaceId)}/attribute-definitions`,
  );
}

export async function createAttributeDefinition(
  workspaceId: string,
  body: {
    scope: string;
    name: string;
    value_type: string;
    required_for_share_modes?: string[];
  },
) {
  return api.post<{ attribute: AttributeDefinitionDto }>(
    `${wsBase(workspaceId)}/attribute-definitions`,
    body,
  );
}

export async function listPrincipalAttributes(workspaceId: string) {
  return api.get<{ attributes: PrincipalAttributeDto[] }>(
    `${wsBase(workspaceId)}/principal-attributes`,
  );
}

export async function upsertPrincipalAttribute(
  workspaceId: string,
  body: {
    principal_kind: string;
    principal_id: string;
    name: string;
    value: unknown;
    source?: string;
  },
) {
  return api.put<{ attribute: PrincipalAttributeDto }>(
    `${wsBase(workspaceId)}/principal-attributes`,
    body,
  );
}

export async function listPolicies(workspaceId: string) {
  return api.get<{ policies: PolicyDto[] }>(`${wsBase(workspaceId)}/policies`);
}

export async function createPolicy(workspaceId: string, body: { name: string }) {
  return api.post<{ policy: PolicyDto }>(`${wsBase(workspaceId)}/policies`, body);
}

export async function publishPolicyVersion(
  workspaceId: string,
  policyId: string,
  body: { cedar_text: string },
) {
  return api.post<{ version: unknown; policy_generation: number }>(
    `${wsBase(workspaceId)}/policies/${policyId}/versions`,
    body,
  );
}

export async function listBreakGlass(workspaceId: string) {
  return api.get<{ sessions: BreakGlassSessionDto[] }>(
    `${wsBase(workspaceId)}/break-glass`,
  );
}

export async function createBreakGlass(
  workspaceId: string,
  body: { reason: string; ttl_minutes?: number; scope_doc_ids?: string[] },
) {
  return api.post<{ session: BreakGlassSessionDto }>(
    `${wsBase(workspaceId)}/break-glass`,
    body,
  );
}

export async function revokeBreakGlass(workspaceId: string, sessionId: string) {
  return api.delete<{ revoked: boolean }>(
    `${wsBase(workspaceId)}/break-glass/${sessionId}`,
  );
}

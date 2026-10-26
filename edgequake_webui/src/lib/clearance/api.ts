import { api } from "@/lib/api/client";
import type { ClearanceRuleBody } from "./rules";
import type {
  ClearanceSettings,
  ClearanceTag,
  MemberRolesResponse,
  SecurityRoleView,
} from "./types";

function base(workspaceId: string) {
  return `/workspaces/${workspaceId}/clearance`;
}

export const clearanceApi = {
  settings: (workspaceId: string) => api.get<ClearanceSettings>(base(workspaceId)),
  enable: (workspaceId: string, strictStart: boolean) =>
    api.post<{ enforcement: boolean }>(base(workspaceId), { strict_start: strictStart }),
  tags: (workspaceId: string) => api.get<ClearanceTag[]>(`${base(workspaceId)}/tags`),
  createTag: (workspaceId: string, value: string) =>
    api.post<ClearanceTag>(`${base(workspaceId)}/tags`, { key: value, value }),
  deleteTag: (workspaceId: string, tagId: string) =>
    api.delete<void>(`${base(workspaceId)}/tags/${tagId}`),
  roles: (workspaceId: string) => api.get<SecurityRoleView[]>(`${base(workspaceId)}/roles`),
  createRole: (workspaceId: string, name: string) =>
    api.post<SecurityRoleView>(`${base(workspaceId)}/roles`, { name, steward: false }),
  saveRules: (workspaceId: string, roleId: string, rules: ClearanceRuleBody[]) =>
    api.put<{ ok: boolean }>(`${base(workspaceId)}/roles/${roleId}/rules`, { rules }),
  memberRoles: (workspaceId: string, userId: string) =>
    api.get<MemberRolesResponse>(`/workspaces/${workspaceId}/members/${userId}/clearance-roles`),
  saveMemberRoles: (workspaceId: string, userId: string, roleIds: string[]) =>
    api.put<{ ok: boolean }>(`/workspaces/${workspaceId}/members/${userId}/clearance-roles`, {
      role_ids: roleIds,
    }),
  simulate: (workspaceId: string, roleIds: string[]) =>
    api.post<{ titles: string[]; count?: number | null }>(`${base(workspaceId)}/simulate`, {
      role_ids: roleIds,
    }),
};

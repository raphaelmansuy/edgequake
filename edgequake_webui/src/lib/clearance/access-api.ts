import { api } from '@/lib/api/client';
import type { AccessDraft, UploadAccess } from './access-draft';
import type { ClassificationWord } from './types';

export interface SecurityTag { id: string; key: string; value: string }
export interface AccessCapabilities {
  can_edit: boolean;
  can_downgrade: boolean;
  can_manage_vocabulary: boolean;
  can_check_member: boolean;
}
export interface AssignmentOptions {
  enforcement: boolean;
  policy_epoch: number;
  break_glass: boolean;
  can_upload: boolean;
  capabilities: AccessCapabilities;
  classifications: ClassificationWord[];
  tags: SecurityTag[];
  next_cursor: string | null;
}
export interface SavedDocumentAccess {
  document_id: string;
  access: UploadAccess;
  label_version: number;
  policy_epoch: number;
  enforcement: boolean;
  capabilities: AccessCapabilities;
  tags: SecurityTag[];
}
export interface AccessPreview {
  allowed: boolean;
  policy_epoch: number;
  items: Array<{ allowed: boolean; requires_reason: boolean; change_kind: string; reason: string | null; caller_can_read?: boolean; member_can_read?: boolean | null; normalized_access?: UploadAccess }>;
}
export interface MutationItem { document_id: string; label_version: number; access: UploadAccess }
export const accessApi = {
  options: (workspace: string, query = '', cursor?: string) => {
    const params = new URLSearchParams({ q: query });
    if (cursor) params.set('cursor', cursor);
    return api.get<AssignmentOptions>(`/workspaces/${workspace}/clearance/assignment-options?${params}`);
  },
  read: (id: string) => api.get<SavedDocumentAccess>(`/documents/${id}/access`),
  preview: (workspace: string, items: Array<{ document_id?: string; label_version?: number; access: UploadAccess }>, epoch: number, checkMember?: string) =>
    api.post<AccessPreview>(`/workspaces/${workspace}/clearance/preview`, {
      mode: items.every(item => !item.document_id) ? 'create' : items.length === 1 ? 'edit' : 'bulk',
      items, expected_policy_epoch: epoch, check_member_id: checkMember,
    }),
  members: (workspace: string, query: string) => api.get<{ members: Array<{user_id: string; name: string}> }>(`/workspaces/${workspace}/clearance/members?q=${encodeURIComponent(query)}`),
  save: (saved: SavedDocumentAccess, draft: AccessDraft, reason: string, operation: string) =>
    api.patch<void>(`/documents/${saved.document_id}/labels`, {
      ...draft, expected_policy_epoch: saved.policy_epoch, reason: reason.trim() || undefined,
    }, { headers: { 'If-Match': `"label-${saved.label_version}"`, 'Idempotency-Key': operation } }),
  bulk: (items: MutationItem[], reason: string, operation: string) =>
    api.post<{ policy_epoch: number }>('/documents/labels/batch', { items, reason: reason.trim() || undefined, operation_id: operation }),
  status: (workspace: string, operation: string) => api.get<{ status: string }>(`/workspaces/${workspace}/clearance/operations/${operation}`),
};

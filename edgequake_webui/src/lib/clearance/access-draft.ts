import type { ClassificationWord } from './types';

export interface AccessDraft {
  classification: ClassificationWord;
  compartment_ids: string[];
}
export interface UploadAccess extends AccessDraft { expected_policy_epoch: number }
export const REVIEW_ACCESS: AccessDraft = { classification: 'unverified', compartment_ids: [] };

export function canonicalAccess(value: AccessDraft): AccessDraft {
  return { classification: value.classification, compartment_ids: [...new Set(value.compartment_ids)].sort() };
}
export function sameAccess(a: AccessDraft, b: AccessDraft): boolean {
  return JSON.stringify(canonicalAccess(a)) === JSON.stringify(canonicalAccess(b));
}
export function accessPayload(value: AccessDraft, epoch: number): UploadAccess {
  return { ...canonicalAccess(value), expected_policy_epoch: epoch };
}
export function accessBroadens(before: AccessDraft, after: AccessDraft): boolean {
  const ranks = ['public', 'internal', 'confidential', 'restricted', 'unverified'];
  return ranks.indexOf(after.classification) < ranks.indexOf(before.classification)
    || before.compartment_ids.some(id => !after.compartment_ids.includes(id));
}

/** Single transport field and serializer; security never travels in generic metadata. */
export function appendUploadAccess(form: FormData, access?: UploadAccess): void {
  if (access) form.append('access', JSON.stringify(accessPayload(access, access.expected_policy_epoch)));
}

'use client';

import { useEffect, useRef, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { LockKeyhole } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useAuthStore } from '@/stores/use-auth-store';
import { accessApi, type AssignmentOptions, type SavedDocumentAccess } from '@/lib/clearance/access-api';
import { accessPayload, REVIEW_ACCESS, sameAccess, type AccessDraft } from '@/lib/clearance/access-draft';
import { useAccessOptions } from '@/lib/clearance/use-access-options';
import { invalidateClearanceViews } from '@/lib/clearance/use-clearance';
import { DocumentAccessFields } from './document-access-fields';

export function BulkDocumentAccessControl({ documentIds }: { documentIds: string[] }) {
  const { t } = useTranslation();
  const workspace = useTenantStore(state => state.selectedWorkspaceId);
  const principal = useAuthStore(state => state.user?.user_id ?? state.user?.id);
  const [open, setOpen] = useState(false);
  const [dirty,setDirty]=useState(false);
  const [busy,setBusy]=useState(false);
  const [discard,setDiscard]=useState(false);
  const requestClose=()=>{if(busy)return;if(dirty)setDiscard(true);else setOpen(false);};
  const options = useAccessOptions(workspace);
  const saved = useQuery({ queryKey: ['bulk-document-access', workspace, principal, [...documentIds].sort()], queryFn: () => Promise.all(documentIds.map(accessApi.read)), enabled: open && Boolean(workspace) && documentIds.length <= 100, retry: false });
  if (!documentIds.length) return null;
  return <>
    <Button variant="outline" size="sm" className="gap-2" disabled={documentIds.length > 100 || !options.data?.capabilities.can_edit || !options.data.enforcement} onClick={() => {setDirty(false);setDiscard(false);setOpen(true);}} data-testid="bulk-access-trigger"><LockKeyhole className="h-3.5 w-3.5" aria-hidden />{t('access.editSelected', 'Edit access ({{count}})', { count: documentIds.length })}</Button>
    <Dialog open={open} onOpenChange={next=>{if(next)setOpen(true);else requestClose();}}><DialogContent showCloseButton={!busy} className="flex max-h-[92dvh] flex-col gap-0 overflow-hidden p-0 sm:max-w-[520px]" data-testid="bulk-access-editor" onEscapeKeyDown={event=>{event.preventDefault();event.stopPropagation();requestClose();}}><DialogHeader className="shrink-0 border-b px-5 py-5 text-left"><DialogTitle>{t('access.bulkTitle', 'Edit selected document access')}</DialogTitle><DialogDescription>{t('access.bulkHelp', 'Every change is checked together. If one document is blocked or stale, none are changed.')}</DialogDescription>
      {discard&&<div role="alert" className="mt-3 rounded-lg border p-3"><p className="text-sm">{t('access.discardPrompt','Discard your unsaved access changes?')}</p><div className="mt-3 flex gap-2"><Button size="sm" variant="outline" onClick={()=>setDiscard(false)}>{t('access.keepEditing','Keep editing')}</Button><Button size="sm" variant="destructive" onClick={()=>setOpen(false)}>{t('access.discard','Discard changes')}</Button></div></div>}
    </DialogHeader>
      {options.data && saved.data && workspace ? <BulkBody key={`${workspace}:${documentIds.join(':')}`} workspace={workspace} saved={saved.data} options={options.data} onDirtyChange={setDirty} onBusyChange={setBusy} onClose={() => setOpen(false)} /> : <div className="p-5 text-sm" role="status">{saved.isError || options.isError ? t('access.checkFailed', 'Access rules could not be checked.') : t('access.loading', 'Checking access…')}</div>}
    </DialogContent></Dialog>
  </>;
}

function BulkBody({ workspace, saved, options, onClose,onDirtyChange,onBusyChange }: { workspace: string; saved: SavedDocumentAccess[]; options: AssignmentOptions; onClose: () => void;onDirtyChange:(dirty:boolean)=>void;onBusyChange:(busy:boolean)=>void }) {
  const { t } = useTranslation();
  const client = useQueryClient();
  const [operation, setOperation] = useState<'add' | 'remove' | 'replace'>('add');
  const [draft, setDraft] = useState<AccessDraft>(REVIEW_ACCESS);
  const [reason, setReason] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(()=>onDirtyChange(!sameAccess(draft,REVIEW_ACCESS)||operation!=='add'||Boolean(reason.trim())),[draft,operation,reason,onDirtyChange]);
  useEffect(()=>onBusyChange(busy),[busy,onBusyChange]);
  const requestKey = useRef({ body: '', id: crypto.randomUUID() });
  const items = saved.map(document => ({ document_id: document.document_id, label_version: document.label_version, access: accessPayload(operation === 'replace' ? draft : {
    classification: document.access.classification,
    compartment_ids: operation === 'add' ? [...new Set([...document.access.compartment_ids, ...draft.compartment_ids])] : document.access.compartment_ids.filter(id => !draft.compartment_ids.includes(id)),
  }, options.policy_epoch) }));
  const preview = useQuery({ queryKey: ['bulk-access-preview', workspace, items], queryFn: () => accessApi.preview(workspace, items, options.policy_epoch), retry: false, enabled: options.enforcement });
  const broadening = preview.data?.items.some(item => item.requires_reason);
  const changed = preview.data?.items.filter(item => item.change_kind !== 'unchanged').length ?? 0;
  async function apply() {
    setBusy(true); setError(null);
    const body = JSON.stringify({ items, reason: reason.trim() });
    if (requestKey.current.body !== body) requestKey.current = { body, id: crypto.randomUUID() };
    try { await accessApi.bulk(items, reason, requestKey.current.id); await invalidateClearanceViews(client, workspace); onClose(); }
    catch (failure) { setError(failure instanceof Error ? failure.message : t('access.saveFailed', 'Could not save document access.')); }
    finally { setBusy(false); }
  }
  return <><div className="space-y-5 overflow-y-auto px-5 py-5" style={{ maxHeight: 'calc(92dvh - 190px)' }}>
    <div className="space-y-2"><Label>{t('access.bulkOperation', 'Operation')}</Label><Select value={operation} onValueChange={value => setOperation(value as typeof operation)} disabled={busy}><SelectTrigger aria-label={t('access.bulkOperation', 'Operation')}><SelectValue /></SelectTrigger><SelectContent><SelectItem value="add">{t('access.addRequired', 'Add required tags')}</SelectItem><SelectItem value="remove">{t('access.removeRequired', 'Remove required tags')}</SelectItem><SelectItem value="replace">{t('access.replaceAccess', 'Replace access')}</SelectItem></SelectContent></Select></div>
    {operation !== 'replace' && <p className="text-xs text-muted-foreground">{t('access.preserveSensitivity', 'Each document keeps its current sensitivity and unrelated tags.')}</p>}
    <DocumentAccessFields workspace={workspace} value={draft} onChange={setDraft} options={options} knownTags={saved.flatMap(item => item.tags)} disabled={busy} showSensitivity={operation === 'replace'} showExplanation={operation === 'replace'} />
    <div className="rounded-lg border bg-muted/20 p-3 text-sm" role="status">{preview.isFetching ? t('access.checking', 'Checking access…') : t('access.bulkPreview', '{{changed}} of {{count}} documents will change.', { changed, count: saved.length })}</div>
    {preview.data && !preview.data.allowed && <p role="alert" className="text-sm text-destructive">{preview.data.items.find(item => !item.allowed)?.reason}</p>}
    {preview.isError && <p role="alert" className="text-sm">{t('access.previewFailed', 'Access could not be checked. Your draft is retained.')}</p>}
    {broadening && <div className="space-y-2"><Label htmlFor="bulk-access-reason">{t('access.reason', 'Reason for change')}</Label><Textarea id="bulk-access-reason" value={reason} onChange={event => setReason(event.target.value)} maxLength={500} placeholder={t('access.reasonLength', '10–500 characters')} /></div>}
    {error && <p role="alert" className="text-sm text-destructive">{error}</p>}
  </div><div className="flex justify-end gap-2 border-t bg-muted/15 px-5 py-3"><Button variant="outline" disabled={busy} onClick={onClose}>{t('common.cancel', 'Cancel')}</Button><Button disabled={busy || !changed || !preview.data?.allowed || preview.isFetching || (broadening && reason.trim().length < 10)} onClick={() => void apply()}>{busy ? t('access.saving', 'Saving…') : t('access.applyCount', 'Apply to {{count}} documents', { count: saved.length })}</Button></div></>;
}

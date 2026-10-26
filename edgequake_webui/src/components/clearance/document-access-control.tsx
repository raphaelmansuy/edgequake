'use client';

import { useRef, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { LockKeyhole, ShieldCheck } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { useAuthStore } from "@/stores/use-auth-store";
import { useTenantStore } from '@/stores/use-tenant-store';
import { accessApi, type AssignmentOptions, type SavedDocumentAccess } from '@/lib/clearance/access-api';
import { accessPayload, sameAccess, type AccessDraft } from '@/lib/clearance/access-draft';
import { useAccessOptions } from '@/lib/clearance/use-access-options';
import { invalidateClearanceViews } from '@/lib/clearance/use-clearance';
import { documentLabelSummary } from '@/lib/clearance/summary';
import { DocumentAccessFields } from './document-access-fields';
import { DocumentMemberCheck } from './document-member-check';

export interface AccessibleDocument { id: string; classification?: string | null; compartments?: Array<{id: string; value: string}> | null }

export function DocumentAccessControl({ document }: { document: AccessibleDocument }) {
  const { t } = useTranslation();
  const workspace = useTenantStore(state => state.selectedWorkspaceId);
  const tenant = useTenantStore(state => state.selectedTenantId);
  const principal = useAuthStore(state => state.user?.user_id ?? state.user?.id);
  const [open, setOpen] = useState(false);
  const options = useAccessOptions(workspace);
  const saved = useQuery({ queryKey: ['document-access', tenant, workspace, principal, document.id], queryFn: () => accessApi.read(document.id), enabled: open && Boolean(workspace), retry: false });
  return <>
    <Button type="button" variant="outline" size="sm" className="h-auto min-h-8 max-w-full gap-2 whitespace-normal text-left text-xs font-normal" onClick={() => setOpen(true)} data-testid="access-property">
      <LockKeyhole className="h-3.5 w-3.5 shrink-0 text-muted-foreground" aria-hidden />
      {document.classification ? documentLabelSummary(document.classification, (document.compartments ?? []).map(tag => tag.value), false) : t('access.viewAccess', 'View document access')}
    </Button>
    {open && options.data && saved.data ? <AccessEditDialog key={`${principal}:${tenant}:${workspace}:${document.id}`} workspace={workspace!} options={options.data} saved={saved.data} onClose={() => setOpen(false)} />
      : <Dialog open={open} onOpenChange={setOpen}><DialogContent><DialogHeader><DialogTitle>{t('access.documentAccess', 'Document access')}</DialogTitle><DialogDescription>{t('access.checkRules', 'Check the rules for this document.')}</DialogDescription></DialogHeader>
        {options.isError || saved.isError ? <div role="alert" className="space-y-3"><p className="text-sm">{t('access.checkFailed', 'Access rules could not be checked.')}</p><Button variant="outline" onClick={() => { void options.refetch(); void saved.refetch(); }}>{t('common.retry', 'Retry')}</Button></div> : <p role="status" className="py-8 text-center text-sm text-muted-foreground">{t('access.loading', 'Checking access…')}</p>}
      </DialogContent></Dialog>}
  </>;
}

function AccessEditDialog({ workspace, options, saved, onClose }: { workspace: string; options: AssignmentOptions; saved: SavedDocumentAccess; onClose: () => void }) {
  const { t } = useTranslation();
  const client = useQueryClient();
  const [draft, setDraft] = useState<AccessDraft>(saved.access);
  const [reason, setReason] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [reconciling,setReconciling]=useState(false);
  const [notice,setNotice]=useState<string|null>(null);
  const [discard, setDiscard] = useState(false);
  const operation = useRef(crypto.randomUUID());
  const dirty = !sameAccess(draft, saved.access);
  const preview = useQuery({
    queryKey: ['document-access-preview', workspace, saved.document_id, saved.label_version, saved.policy_epoch, draft],
    queryFn: () => accessApi.preview(workspace, [{ document_id: saved.document_id, label_version: saved.label_version, access: accessPayload(draft, saved.policy_epoch) }], saved.policy_epoch),
    enabled: dirty && options.enforcement && saved.capabilities.can_edit, retry: false,
  });
  const broadening = preview.data?.items.some(item => item.requires_reason) ?? false;
  const blocked = !saved.capabilities.can_edit || !saved.enforcement;
  const reasonValid = !broadening || (reason.trim().length >= 10 && reason.trim().length <= 500);
  async function save() {
    if (saving || reconciling || error || !preview.data?.allowed || !reasonValid) return;
    setSaving(true); setError(null);
    try {
      await accessApi.save(saved, draft, reason, operation.current);
      await invalidateClearanceViews(client, workspace);
      onClose();
    } catch (failure) { setError(failure instanceof Error ? failure.message : t('access.saveFailed', 'Could not save document access.')); }
    finally { setSaving(false); }
  }
  async function reconcile() {
    setReconciling(true);
    try {
      const status=await accessApi.status(workspace,operation.current);
      if(status.status==='committed') {await invalidateClearanceViews(client,workspace);onClose();return;}
      if(status.status==='pending') {setError(t('access.operationPending','This change is still in progress. Check status again before retrying.'));return;}
      await client.invalidateQueries({queryKey:['document-access']});
      await client.invalidateQueries({queryKey:['document-access-options']});
      await client.invalidateQueries({queryKey:['document-access-preview']});
      setError(null);setNotice(t('access.latestReviewed','Latest saved access loaded. Your draft is retained; review it before saving.'));
    } catch(failure) {setError(failure instanceof Error?failure.message:t('access.checkFailed','Access rules could not be checked.'));}
    finally {setReconciling(false);}
  }
  return <Dialog open onOpenChange={next => { if (!next && !saving) { if (dirty) setDiscard(true); else onClose(); } }}>
    <DialogContent className="max-h-[90dvh] gap-0 overflow-hidden p-0 sm:max-w-[480px]" data-testid="document-access-editor" onEscapeKeyDown={event => { event.preventDefault(); event.stopPropagation(); if (!saving) { if (dirty) setDiscard(true); else onClose(); } }}>
      <DialogHeader className="border-b px-5 py-5 text-left"><DialogTitle className="flex items-center gap-2 text-base"><ShieldCheck className="h-4 w-4 text-muted-foreground" aria-hidden />{t('access.documentAccess', 'Document access')}</DialogTitle><DialogDescription>{t('access.editHint', 'Choose a sensitivity and the security tags readers need.')}</DialogDescription></DialogHeader>
      <div className="space-y-4 overflow-y-auto px-5 py-5 scroll-pb-24" style={{ maxHeight: 'calc(90dvh - 190px)' }}>
        <DocumentAccessFields workspace={workspace} value={draft} onChange={value => { setDraft(value); setError(null); operation.current = crypto.randomUUID(); }} options={options} knownTags={saved.tags} disabled={blocked || saving} />
        {saved.capabilities.can_check_member && saved.enforcement && <DocumentMemberCheck workspace={workspace} saved={saved} draft={draft} />}
        {dirty && <p className="text-xs text-muted-foreground" role="status">{t('access.unsaved', 'Unsaved changes')}{preview.isFetching ? ` · ${t('access.checking', 'Checking access…')}` : ''}</p>}
        {preview.isError && <div role="alert" className="rounded-lg border border-destructive/40 p-3 text-sm"><p>{t('access.previewFailed', 'Access could not be checked. Your draft is retained.')}</p><Button type="button" variant="outline" size="sm" className="mt-2" onClick={() => void preview.refetch()}>{t('common.retry', 'Retry')}</Button></div>}
        {preview.data && !preview.data.allowed && <p role="alert" className="rounded-lg border border-amber-500/30 bg-amber-500/5 p-3 text-sm">{preview.data.items.find(item => !item.allowed)?.reason}</p>}
        {broadening && <div className="space-y-2 rounded-lg border border-amber-500/30 bg-amber-500/5 p-3.5"><p className="text-sm font-medium">{t('access.broadening', 'This change broadens access')}</p><div className="rounded-md border bg-background/50 p-2.5 text-xs leading-relaxed"><p>{t('access.savedValue', 'Saved')}: {documentLabelSummary(saved.access.classification, saved.tags.map(tag => tag.value), true)}</p><p className="mt-1">{t('access.draftValue', 'Draft')}: {t(`access.${draft.classification}`, draft.classification)} · {t('access.requiredTagCount', '{{count}} required tags', {count:draft.compartment_ids.length})}</p></div><p className="text-xs leading-relaxed text-muted-foreground">{t('access.reasonHelp', 'A security steward must provide a reason. It is recorded in the security audit.')}</p><Label htmlFor="access-change-reason">{t('access.reason', 'Reason for change')}</Label><Textarea id="access-change-reason" value={reason} onChange={event => {setReason(event.target.value);operation.current=crypto.randomUUID();}} maxLength={500} placeholder={t('access.reasonPlaceholder', 'Explain why this change is appropriate…')} aria-describedby="access-reason-hint" /><p id="access-reason-hint" className="text-xs text-muted-foreground">{t('access.reasonLength', '10–500 characters')}</p></div>}
        {notice && <div role="status" className="rounded-lg border p-3 text-xs leading-relaxed"><p>{notice}</p><p className="mt-1">{t('access.savedValue','Saved')}: {documentLabelSummary(saved.access.classification,saved.tags.map(tag=>tag.value),true)}</p></div>}
        {error && <div role="alert" className="space-y-2 rounded-lg border border-destructive/40 p-3 text-sm"><p>{error}</p><Button variant="outline" size="sm" disabled={reconciling} onClick={()=>void reconcile()}>{reconciling?t('access.checking','Checking access…'):t('access.checkStatus', 'Check status / reload access')}</Button></div>}
        {blocked && <p className="text-xs text-muted-foreground">{!saved.enforcement ? t('access.rulesOff', 'Document access rules are off for this workspace.') : t('access.readOnly', 'Your role cannot change document access.')}</p>}
        {discard && <div role="alert" className="rounded-lg border p-3"><p className="text-sm">{t('access.discardPrompt', 'Discard your unsaved access changes?')}</p><div className="mt-3 flex gap-2"><Button variant="outline" size="sm" onClick={() => setDiscard(false)}>{t('access.keepEditing', 'Keep editing')}</Button><Button variant="destructive" size="sm" onClick={onClose}>{t('access.discard', 'Discard changes')}</Button></div></div>}
      </div>
      <DialogFooter className="border-t bg-muted/15 px-5 py-3"><Button type="button" variant="outline" disabled={saving||reconciling} onClick={onClose}>{t('common.cancel', 'Cancel')}</Button><Button type="button" disabled={blocked || !dirty || saving || reconciling || Boolean(error) || preview.isFetching || !preview.data?.allowed || !reasonValid} onClick={() => void save()}>{saving ? t('access.saving', 'Saving…') : broadening ? t('access.saveChange', 'Save access change') : t('access.save', 'Save access')}</Button></DialogFooter>
    </DialogContent>
  </Dialog>;
}

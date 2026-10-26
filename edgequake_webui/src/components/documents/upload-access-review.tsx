'use client';

import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { FileText, ShieldCheck } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { DocumentAccessFields } from '@/components/clearance/document-access-fields';
import { useAccessOptions } from '@/lib/clearance/use-access-options';
import { accessApi, type AssignmentOptions } from '@/lib/clearance/access-api';
import { accessPayload, REVIEW_ACCESS, type AccessDraft, type UploadAccess } from '@/lib/clearance/access-draft';

export interface UploadAccessReviewProps {
  files: File[]; workspace: string; onClose: () => void;
  onUpload: (files: File[], access: Map<File, UploadAccess>) => Promise<void>;
}

export function UploadAccessReview(props: UploadAccessReviewProps) {
  const { t } = useTranslation();
  const options = useAccessOptions(props.workspace);
  const [busy,setBusy]=useState(false);
  return <Dialog open onOpenChange={open => { if (!open && !busy) props.onClose(); }}><DialogContent showCloseButton={!busy} className="max-h-[92dvh] gap-0 overflow-hidden p-0 sm:max-w-[640px]" data-testid="upload-access-review">
    <DialogHeader className="border-b px-5 py-5 text-left"><DialogTitle className="flex items-center gap-2 text-base"><ShieldCheck className="h-4 w-4 text-muted-foreground" aria-hidden />{t('access.reviewUpload', 'Review upload')}</DialogTitle><DialogDescription>{props.files.length===1?t('access.reviewSingleUploadHelp','1 file selected. Set access before processing starts.'):t('access.reviewUploadHelp', '{{count}} files selected. Set access before processing starts.', { count: props.files.length })}</DialogDescription></DialogHeader>
    {options.data ? <UploadReviewBody key={props.workspace} {...props} options={options.data} busy={busy} onBusyChange={setBusy} />
      : <div className="space-y-3 p-5">{options.isError ? <><p role="alert" className="text-sm">{t('access.checkFailed', 'Access rules could not be checked.')}</p><Button variant="outline" onClick={() => void options.refetch()}>{t('common.retry', 'Retry')}</Button></> : <p role="status" className="py-8 text-center text-sm text-muted-foreground">{t('access.loading', 'Checking access…')}</p>}</div>}
  </DialogContent></Dialog>;
}

function UploadReviewBody({ files, workspace, options, onClose, onUpload, busy, onBusyChange:setBusy }: UploadAccessReviewProps & { options: AssignmentOptions; busy:boolean; onBusyChange:(busy:boolean)=>void }) {
  const { t } = useTranslation();
  const [shared, setShared] = useState<AccessDraft>(REVIEW_ACCESS);
  const [overrides, setOverrides] = useState(new Map<File, AccessDraft>());
  const [editing, setEditing] = useState<File | null>(null);
  const [epoch,setEpoch]=useState(options.policy_epoch);
  const stale=epoch!==options.policy_epoch;
  const [error, setError] = useState<string | null>(null);
  const items = files.map(file => ({ access: accessPayload(overrides.get(file) ?? shared, epoch) }));
  const preview = useQuery({ queryKey: ['upload-access-preview', workspace, options.policy_epoch, items],
    queryFn: () => accessApi.preview(workspace, items, options.policy_epoch),
    enabled: options.enforcement && options.can_upload && !stale, retry: false });
  const current = editing ? overrides.get(editing) ?? shared : shared;
  const change = (value: AccessDraft) => { if (editing) setOverrides(previous => new Map(previous).set(editing, value)); else setShared(value); };
  async function upload() {
    setBusy(true); setError(null);
    try { await onUpload(files, options.enforcement ? new Map(files.map(file => [file, accessPayload(overrides.get(file) ?? shared, epoch)])) : new Map()); onClose(); }
    catch (failure) { setError(failure instanceof Error ? failure.message : t('access.uploadFailed', 'Upload could not start. Your selections are retained.')); }
    finally { setBusy(false); }
  }
  return <>
    <div className="space-y-5 overflow-y-auto px-5 py-5 scroll-pb-24" style={{ maxHeight: 'calc(92dvh - 180px)' }}>
      <div className="flex min-w-0 items-start justify-between gap-3"><div className="min-w-0"><h3 className="break-words text-sm font-medium">{editing ? editing.name : t('access.allFiles', 'Access for all files')}</h3><p className="mt-1 text-xs leading-relaxed text-muted-foreground">{editing ? t('access.overrideHelp', 'This file has its own access settings.') : t('access.batchHelp', 'Shared settings apply to files without an override.')}</p></div>{editing && <Button variant="outline" size="sm" className="shrink-0" onClick={() => setEditing(null)}>{t('access.batchDefaults', 'Batch defaults')}</Button>}</div>
      <DocumentAccessFields workspace={workspace} value={current} onChange={change} options={options} disabled={busy} />
      <section className="overflow-hidden rounded-lg border" aria-label={t('access.files', 'Selected files')}><div className="border-b bg-muted/25 px-3 py-2 text-xs font-medium text-muted-foreground">{t('access.files', 'Selected files')}</div>{files.map(file => <div key={`${file.name}:${file.lastModified}`} className="flex min-h-14 items-center gap-3 border-b px-3 py-2 last:border-0">
        <FileText className="h-4 w-4 shrink-0 text-muted-foreground" aria-hidden /><div className="min-w-0 flex-1"><p className="truncate text-sm" title={file.name}>{file.name}</p><p className="mt-0.5 text-xs text-muted-foreground">{overrides.has(file) ? t('access.override', 'Per-file override') : t('access.usesBatch', 'Uses batch access')}</p></div>
        {overrides.has(file) && <Button variant="ghost" size="sm" className="text-xs" disabled={busy} onClick={() => { setOverrides(previous => { const next = new Map(previous); next.delete(file); return next; }); if (editing === file) setEditing(null); }}>{t('common.reset', 'Reset')}</Button>}
        <Button variant="outline" size="sm" className="text-xs" disabled={busy} onClick={() => setEditing(file)}>{t('access.customize', 'Customize')}</Button>
      </div>)}</section>
      {options.enforcement && <p role="status" className="text-xs text-muted-foreground">{preview.isFetching ? t('access.checking', 'Checking access…') : preview.data?.allowed ? t('access.readyUpload', 'Access checked. Ready to upload.') : preview.data?.items.find(item => !item.allowed)?.reason}</p>}
      {preview.isError && <div role="alert" className="space-y-2 text-sm"><p>{t('access.previewFailed', 'Access could not be checked. Your draft is retained.')}</p><Button variant="outline" size="sm" onClick={() => void preview.refetch()}>{t('common.retry', 'Retry')}</Button></div>}
      {stale && <div role="alert" className="space-y-2 rounded-lg border border-amber-500/30 p-3 text-sm"><p>{t('access.policyChangedDraft', 'Access rules changed. Your draft is retained.')}</p><Button variant="outline" size="sm" onClick={()=>setEpoch(options.policy_epoch)}>{t('access.recheckRules', 'Review latest rules')}</Button></div>}
      {error && <p role="alert" className="text-sm text-destructive">{error}</p>}
    </div>
    <div className="flex items-center justify-end gap-2 border-t bg-muted/15 px-5 py-3"><Button variant="outline" disabled={busy} onClick={onClose}>{t('common.cancel', 'Cancel')}</Button><Button disabled={busy || stale || !options.can_upload || (options.enforcement && (preview.isFetching || !preview.data?.allowed))} onClick={() => void upload()}>{busy ? t('access.starting', 'Starting upload…') : files.length===1?t('access.uploadOne','Upload 1 file'):t('access.uploadCount', 'Upload {{count}} files', { count: files.length })}</Button></div>
  </>;
}

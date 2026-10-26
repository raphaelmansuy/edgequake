'use client';
import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { useAuthStore } from '@/stores/use-auth-store';
import { accessApi, type SavedDocumentAccess } from '@/lib/clearance/access-api';
import { accessPayload, type AccessDraft } from '@/lib/clearance/access-draft';

export function DocumentMemberCheck({workspace,saved,draft}:{workspace:string;saved:SavedDocumentAccess;draft:AccessDraft}) {
  const {t}=useTranslation();
  const [query,setQuery]=useState('');
  const [member,setMember]=useState<{user_id:string;name:string}|null>(null);
  const principal=useAuthStore(state=>state.user?.user_id??state.user?.id);
  const members=useQuery({queryKey:['access-member-search',workspace,principal,query],queryFn:()=>accessApi.members(workspace,query),enabled:query.trim().length>=2,retry:false});
  const check=useQuery({queryKey:['access-member-check',workspace,principal,member?.user_id,saved.label_version,saved.policy_epoch,draft],queryFn:()=>accessApi.preview(workspace,[{document_id:saved.document_id,label_version:saved.label_version,access:accessPayload(draft,saved.policy_epoch)}],saved.policy_epoch,member!.user_id),enabled:Boolean(member),retry:false});
  return <details className="rounded-lg border p-3.5" data-testid="access-member-check"><summary className="cursor-pointer text-xs font-medium">{t('access.checkMember','Check a workspace member')}</summary><div className="mt-3 space-y-3">
    <p className="text-xs text-muted-foreground">{t('access.memberCheckHelp','Security stewards can check one member against this draft. Results do not grant access.')}</p>
    <div className="space-y-2"><Label htmlFor="access-member-search">{t('access.findMember','Find a member')}</Label><Input id="access-member-search" value={query} onChange={event=>{setQuery(event.target.value);setMember(null);}} placeholder={t('access.memberSearchPlaceholder','Name or email (at least 2 characters)')} /></div>
    {!member && members.data?.members.map(item=><Button key={item.user_id} variant="ghost" className="h-auto min-h-10 w-full justify-start whitespace-normal text-left" onClick={()=>setMember(item)}>{item.name}</Button>)}
    {members.isFetching&&<p role="status" className="text-xs">{t('access.searchingMembers','Searching members…')}</p>}
    {members.isError&&<p role="alert" className="text-xs">{t('access.memberSearchFailed','Members could not be checked. Try again.')}</p>}
    {!members.isFetching&&query.trim().length>=2&&members.data?.members.length===0&&<p className="text-xs text-muted-foreground">{t('access.noMembers','No matching active members.')}</p>}
    {member&&<div role="status" className="rounded-md bg-muted/30 p-3 text-sm"><span className="font-medium">{member.name}</span><p className="mt-1">{check.isFetching?t('access.checking','Checking access…'):check.isError?t('access.memberSearchFailed','Members could not be checked. Try again.'):check.data?.items[0]?.member_can_read?t('access.memberAllowed','Can read under this draft.'):t('access.memberDenied','Cannot read under this draft.')}</p></div>}
  </div></details>;
}

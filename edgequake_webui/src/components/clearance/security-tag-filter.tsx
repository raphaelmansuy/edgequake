'use client';
import { useTranslation } from 'react-i18next';
import { useTenantStore } from '@/stores/use-tenant-store';
import { Button } from '@/components/ui/button';
import { Popover,PopoverContent,PopoverTrigger } from '@/components/ui/popover';
import { useAccessOptions } from '@/lib/clearance/use-access-options';
import { DocumentAccessFields } from './document-access-fields';

export function SecurityTagFilter({ value,onChange }:{value:string[];onChange:(ids:string[])=>void}){
  const {t}=useTranslation();
  const workspace=useTenantStore(state=>state.selectedWorkspaceId);
  const options=useAccessOptions(workspace);
  if(!options.data?.enforcement)return null;
  return <Popover><PopoverTrigger asChild><Button variant="outline" size="sm" data-testid="security-tag-filter">{t('access.filterTags','Security tags')} {value.length ? `(${value.length})` : ''}</Button></PopoverTrigger>
    <PopoverContent align="start" className="w-[min(390px,calc(100vw-2rem))] space-y-3"><p className="text-sm font-medium">{t('access.filterAllTags','Documents with every selected security tag')}</p><DocumentAccessFields workspace={workspace} value={{classification:'unverified',compartment_ids:value}} onChange={draft=>onChange(draft.compartment_ids)} options={options.data} showSensitivity={false} showExplanation={false}/>{value.length>0&&<Button variant="ghost" size="sm" onClick={()=>onChange([])}>{t('access.clearFilter','Clear security tag filter')}</Button>}</PopoverContent></Popover>;
}

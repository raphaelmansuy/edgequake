'use client';

import { useId, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Check, LockKeyhole, Search, ShieldCheck, X } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import type { AccessDraft } from '@/lib/clearance/access-draft';
import type { AssignmentOptions, SecurityTag } from '@/lib/clearance/access-api';
import { useSecurityTags } from '@/lib/clearance/use-access-options';
import type { ClassificationWord } from '@/lib/clearance/types';

export function DocumentAccessFields({ workspace, value, onChange, options, knownTags = [], disabled = false, showSensitivity = true, showExplanation = true }: {
  workspace: string | null | undefined; value: AccessDraft; onChange: (value: AccessDraft) => void;
  options: AssignmentOptions; knownTags?: SecurityTag[]; disabled?: boolean; showSensitivity?: boolean; showExplanation?: boolean;
}) {
  const { t } = useTranslation();
  const sensitivityId = useId();
  const [search, setSearch] = useState('');
  const [open, setOpen] = useState(false);
  const tags = useSecurityTags(workspace, search, open && options.enforcement);
  const catalog = useRef(new Map<string, SecurityTag>());
  const available = tags.data?.pages.flatMap(page => page.tags) ?? options.tags;
  for (const tag of [...options.tags, ...knownTags, ...available]) catalog.current.set(tag.id, tag);
  const nameOf = (id: string) => catalog.current.get(id)?.value ?? t('access.unknownTag', 'Unavailable security tag');
  const toggle = (id: string) => onChange({ ...value, compartment_ids: value.compartment_ids.includes(id)
    ? value.compartment_ids.filter(item => item !== id) : [...value.compartment_ids, id] });
  const locked = disabled || !options.enforcement;
  const sensitivities = [...new Set<ClassificationWord>(['unverified', ...options.classifications, value.classification])];

  return <div className="space-y-5" data-testid="document-access-fields">
    {showSensitivity && <div className="space-y-2">
      <Label htmlFor={sensitivityId} className="text-xs font-medium text-foreground/80">{t('access.sensitivity', 'Sensitivity')}</Label>
      <Select value={value.classification} onValueChange={classification => onChange({ ...value, classification: classification as ClassificationWord })} disabled={locked}>
        <SelectTrigger id={sensitivityId} className="h-10 w-full rounded-lg" aria-label={t('access.sensitivity', 'Sensitivity')}><SelectValue /></SelectTrigger>
        <SelectContent>{sensitivities.map(word => <SelectItem key={word} value={word}>
          {word === 'unverified' ? t('access.needsReview', 'Needs review') : t(`access.${word}`, word.charAt(0).toUpperCase() + word.slice(1))}
        </SelectItem>)}</SelectContent>
      </Select>
      {value.classification === 'public' && <p className="text-xs leading-relaxed text-muted-foreground">{t('access.publicHint', 'Public is a sensitivity level inside this workspace. It does not create a public link.')}</p>}
    </div>}

    <div className="space-y-2">
      <Label className="text-xs font-medium text-foreground/80">{t('access.securityTags', 'Required security tags')}</Label>
      <div className="flex min-h-10 flex-wrap items-center gap-2 rounded-lg border bg-muted/15 p-2">
        {value.compartment_ids.map(id => <span key={id} className="inline-flex max-w-full items-center gap-1.5 rounded-md border bg-background py-1 pl-2 text-xs">
          <LockKeyhole className="h-3 w-3 shrink-0 text-muted-foreground" aria-hidden />
          <span className="break-words">{nameOf(id)}</span>
          <Button type="button" variant="ghost" size="icon" className="mr-0.5 h-6 w-6 shrink-0" disabled={locked} onClick={() => toggle(id)} aria-label={t('access.removeTag', 'Remove security tag {{name}}', { name: nameOf(id) })}><X className="h-3 w-3" /></Button>
        </span>)}
        <Popover open={open} onOpenChange={setOpen}>
          <PopoverTrigger asChild><Button type="button" variant="ghost" size="sm" className="h-8 gap-1.5 px-2 text-xs" disabled={locked} data-testid="security-tag-picker-trigger">{t('access.addTags', '+ Add security tags')}</Button></PopoverTrigger>
          <PopoverContent align="start" className="w-[min(360px,calc(100vw-3rem))] p-0" onEscapeKeyDown={event => event.stopPropagation()}>
            <div className="relative border-b p-3"><Search className="absolute left-6 top-6 h-4 w-4 text-muted-foreground" aria-hidden /><Input autoFocus value={search} onChange={event => setSearch(event.target.value)} className="h-10 pl-9" placeholder={t('access.searchTags', 'Search security tags…')} aria-label={t('access.searchTags', 'Search security tags…')} /></div>
            <div className="max-h-64 overflow-y-auto p-2">
              {value.compartment_ids.length > 0 && <div className="mb-2 border-b pb-2"><p className="px-2 py-1 text-xs font-medium text-muted-foreground">{t('access.selectedTags', 'Selected security tags')}</p>{value.compartment_ids.map(id => <TagChoice key={id} name={nameOf(id)} checked onChange={() => toggle(id)} />)}</div>}
              {tags.isLoading && available.length === 0 && <p className="p-3 text-sm text-muted-foreground" role="status">{t('access.loadingTags', 'Loading security tags…')}</p>}
              {tags.isError ? <div className="p-3" role="alert"><p className="text-sm">{t('access.tagsFailed', 'Security tags could not load.')}</p><Button type="button" variant="outline" size="sm" className="mt-2" onClick={() => void tags.refetch()}>{t('common.retry', 'Retry')}</Button></div>
                : available.filter(tag => !value.compartment_ids.includes(tag.id)).map(tag => <TagChoice key={tag.id} name={tag.value} checked={false} onChange={() => toggle(tag.id)} />)}
              {!tags.isLoading && !tags.isError && available.length === 0 && <p className="p-3 text-sm text-muted-foreground">{t('access.noTags', 'No matching security tags are available to you.')}</p>}
              {tags.hasNextPage && <Button type="button" variant="ghost" className="w-full" onClick={() => void tags.fetchNextPage()} disabled={tags.isFetchingNextPage}>{t('access.loadMore', 'Load more')}</Button>}
            </div>
            <div className="flex items-center justify-between border-t px-3 py-2"><span className="text-xs text-muted-foreground">{t('access.everyTag', 'Readers need every selected tag.')}</span><Button type="button" size="sm" variant="outline" onClick={() => setOpen(false)}>{t('common.done', 'Done')}</Button></div>
          </PopoverContent>
        </Popover>
      </div>
      <p className="text-xs leading-relaxed text-muted-foreground">{t('access.everyTagHelp', 'Security tags control access. Readers must hold every selected tag.')}</p>
    </div>

    {showExplanation && <div className="rounded-lg border bg-muted/25 p-3.5" data-testid="access-explanation">
      <p className="flex items-center gap-2 text-xs font-medium"><ShieldCheck className="h-4 w-4 text-muted-foreground" aria-hidden />{t('access.whoCanAccess', 'Who can access')}</p>
      <p className="mt-2 text-sm leading-relaxed">{!options.enforcement ? t('access.rulesOff', 'Document access rules are off for this workspace.')
        : value.classification === 'unverified' ? t('access.reviewAudience', 'You and authorized security stewards, until reviewed.')
          : <>{t('access.rankAudience', 'Workspace members with {{rank}} access or higher', { rank: value.classification.charAt(0).toUpperCase() + value.classification.slice(1) })}{value.compartment_ids.length ? ` ${t('access.andEveryTag', 'who hold every required tag')}: ${value.compartment_ids.map(nameOf).join(' AND ')}.` : '.'}</>}</p>
      {options.enforcement && value.classification !== 'unverified' && <p className="mt-1.5 text-xs leading-relaxed text-muted-foreground">{t('access.stewardAudience', 'Authorized security stewards may also have access. Workspace permissions still apply.')}</p>}
      {options.break_glass && <p className="mt-2 text-xs text-amber-700 dark:text-amber-300">{t('access.emergency', 'Emergency access is active. Access is audited.')}</p>}
    </div>}
  </div>;
}

function TagChoice({ name, checked, onChange }: { name: string; checked: boolean; onChange: () => void }) {
  return <label className="flex min-h-11 cursor-pointer items-center gap-3 rounded-md px-2 py-2 text-sm hover:bg-muted">
    <Checkbox checked={checked} onCheckedChange={onChange} aria-label={name} />
    <span className="min-w-0 flex-1 break-words">{name}</span>{checked && <Check className="h-3.5 w-3.5 text-muted-foreground" aria-hidden />}
  </label>;
}

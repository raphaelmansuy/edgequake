/**
 * Global ⌘K command palette — SPEC-155 LAW-155-6 / F-155-S01, SPEC-168.
 *
 * Pages come from the shared destinations registry (with their `g` chord
 * hints); documents are searched live once two characters are typed.
 */
'use client';

import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
} from '@/components/ui/command';
import { Kbd, KeyCombo } from '@/components/shared/kbd';
import { useDebounce } from '@/hooks/use-debounce';
import { searchDocuments } from '@/lib/api/edgequake/documents';
import { toggleNavigation } from '@/lib/keyboard/nav-drawer';
import { DESTINATIONS } from '@/lib/keyboard/destinations';
import { beginNewQueryConversation } from '@/lib/query/query-handoff';
import { useKeyboardShortcutsContext } from '@/providers/keyboard-shortcuts-provider';
import { useSettingsStore } from '@/stores/use-settings-store';
import { useQuery } from '@tanstack/react-query';
import {
  Keyboard,
  Laptop,
  Moon,
  PanelLeft,
  Plus,
  Sun,
  FileText,
  Loader2,
  type LucideIcon,
} from 'lucide-react';
import { useTheme } from 'next-themes';
import { useRouter } from 'next/navigation';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

const MIN_DOCUMENT_QUERY = 2;

function PaletteFooter() {
  const { t } = useTranslation();
  return (
    <div
      aria-hidden="true"
      className="flex items-center gap-4 border-t px-3 py-2 text-xs text-muted-foreground"
    >
      <span className="inline-flex items-center gap-1.5">
        <KeyCombo keys={['↑', '↓']} /> {t('command.hintNavigate', 'navigate')}
      </span>
      <span className="inline-flex items-center gap-1.5">
        <Kbd>↵</Kbd> {t('command.hintOpen', 'open')}
      </span>
      <span className="ml-auto inline-flex items-center gap-1.5">
        <Kbd>Esc</Kbd> {t('command.hintClose', 'close')}
      </span>
    </div>
  );
}

function useDocumentMatches(search: string, enabled: boolean) {
  const debounced = useDebounce(search.trim(), 200);
  const active = enabled && debounced.length >= MIN_DOCUMENT_QUERY;
  const result = useQuery({
    queryKey: ['palette-documents', debounced],
    queryFn: () => searchDocuments({ q: debounced, page_size: 6 }),
    enabled: active,
    staleTime: 30_000,
    retry: false,
  });
  return {
    items: active ? (result.data?.items ?? []) : [],
    pending: active && (result.isFetching || debounced !== search.trim()),
  };
}

export function CommandPalette() {
  const { t } = useTranslation();
  const router = useRouter();
  const { searchOpen, closeSearch, openHelp, onCloseAutoFocus } = useKeyboardShortcutsContext();
  const { setTheme } = useTheme();
  const toggleSidebar = useSettingsStore((s) => s.toggleSidebar);
  const [search, setSearch] = useState('');
  const docs = useDocumentMatches(search, searchOpen);

  // Fresh palette every time it opens (derive-from-prop; no effect round-trip).
  const [wasOpen, setWasOpen] = useState(searchOpen);
  if (wasOpen !== searchOpen) {
    setWasOpen(searchOpen);
    if (!searchOpen) setSearch('');
  }

  const run = (action: () => void) => () => {
    closeSearch();
    action();
  };
  const go = (href: string) => run(() => router.push(href));

  const themeActions: Array<{ id: string; icon: LucideIcon; key: string; fallback: string; value: string }> = [
    { id: 'light', icon: Sun, key: 'command.themeLight', fallback: 'Switch to light theme', value: 'light' },
    { id: 'dark', icon: Moon, key: 'command.themeDark', fallback: 'Switch to dark theme', value: 'dark' },
    { id: 'system', icon: Laptop, key: 'command.themeSystem', fallback: 'Use system theme', value: 'system' },
  ];

  return (
    <CommandDialog
      open={searchOpen}
      onCloseAutoFocus={onCloseAutoFocus}
      onOpenChange={(open) => (!open ? closeSearch() : undefined)}
      title={t('command.title', 'Command palette')}
    >
      <CommandInput
        data-testid="command-palette-input"
        value={search}
        onValueChange={setSearch}
        placeholder={t('command.placeholder', 'Search pages, documents and actions…')}
      />
      <CommandList className="max-h-[min(24rem,60dvh)]">
        <CommandEmpty>{t('command.empty', 'Nothing matches')}</CommandEmpty>

        <CommandGroup heading={t('command.navigate', 'Go to')}>
          {DESTINATIONS.map((d) => {
            const label = t(d.labelKey, d.fallback);
            return (
              <CommandItem key={d.href} value={`${label} ${d.href}`} onSelect={go(d.href)}>
                <d.icon aria-hidden="true" />
                <span>{label}</span>
                <span className="ml-auto hidden sm:inline">
                  <KeyCombo keys={['G', d.chord.toUpperCase()]} kind="sequence" />
                </span>
              </CommandItem>
            );
          })}
        </CommandGroup>

        {(docs.items.length > 0 || docs.pending) && (
          <>
            <CommandSeparator />
            <CommandGroup heading={t('command.documents', 'Documents')} forceMount>
              {docs.items.map((doc) => (
                <CommandItem
                  key={doc.id}
                  forceMount
                  value={`doc-${doc.id} ${doc.title}`}
                  onSelect={go(`/documents/${doc.id}`)}
                >
                  <FileText aria-hidden="true" />
                  <span className="truncate">{doc.title}</span>
                </CommandItem>
              ))}
              {docs.pending && docs.items.length === 0 && (
                <div className="flex items-center gap-2 px-2 py-3 text-sm text-muted-foreground" role="status">
                  <Loader2 className="h-4 w-4 animate-spin" aria-hidden="true" />
                  {t('command.searching', 'Searching documents…')}
                </div>
              )}
            </CommandGroup>
          </>
        )}

        <CommandSeparator />
        <CommandGroup heading={t('command.actions', 'Actions')}>
          <CommandItem
            onSelect={run(() => {
              beginNewQueryConversation();
              router.push('/query');
            })}
          >
            <Plus aria-hidden="true" />
            {t('command.newConversation', 'New conversation')}
          </CommandItem>
          <CommandItem onSelect={run(() => toggleNavigation(toggleSidebar))}>
            <PanelLeft aria-hidden="true" />
            <span>{t('command.toggleSidebar', 'Collapse or expand sidebar')}</span>
            <span className="ml-auto hidden sm:inline">
              <KeyCombo keys={['[']} kind="single" />
            </span>
          </CommandItem>
          {themeActions.map((a) => (
            <CommandItem key={a.id} onSelect={run(() => setTheme(a.value))}>
              <a.icon aria-hidden="true" />
              {t(a.key, a.fallback)}
            </CommandItem>
          ))}
          <CommandItem onSelect={run(openHelp)}>
            <Keyboard aria-hidden="true" />
            <span>{t('command.shortcuts', 'Keyboard shortcuts')}</span>
            <span className="ml-auto hidden sm:inline">
              <KeyCombo keys={['?']} kind="single" />
            </span>
          </CommandItem>
        </CommandGroup>
      </CommandList>
      <PaletteFooter />
    </CommandDialog>
  );
}

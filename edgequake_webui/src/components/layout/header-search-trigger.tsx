'use client';

/**
 * SPEC-168 — discoverable entry point for the ⌘K palette.
 * Pill with shortcut hint on wide screens, icon button below `lg`.
 * Rendered synchronously (no ClientOnly) so the header never reflows.
 */
import { KeyCombo } from '@/components/shared/kbd';
import { Button } from '@/components/ui/button';
import { useKeyboardShortcutsContext } from '@/providers/keyboard-shortcuts-provider';
import { Search } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export function HeaderSearchTrigger() {
  const { t } = useTranslation();
  const { openSearch } = useKeyboardShortcutsContext();
  const label = t('command.openPalette', 'Search or jump to…');

  return (
    <>
      <Button
        type="button"
        variant="outline"
        onClick={openSearch}
        data-testid="header-search-trigger"
        aria-label={label}
        aria-keyshortcuts="Control+K Meta+K"
        className="hidden h-8 w-full max-w-sm justify-start gap-2 px-2.5 text-sm font-normal text-muted-foreground shadow-none lg:flex"
      >
        <Search className="h-4 w-4 shrink-0" aria-hidden="true" />
        <span className="truncate">{label}</span>
        <KeyCombo keys={['Mod', 'K']} className="ml-auto shrink-0" />
      </Button>
      <Button
        type="button"
        variant="ghost"
        size="icon"
        onClick={openSearch}
        aria-label={label}
        aria-keyshortcuts="Control+K Meta+K"
        className="h-8 w-8 lg:hidden"
      >
        <Search className="h-4 w-4" aria-hidden="true" />
      </Button>
    </>
  );
}

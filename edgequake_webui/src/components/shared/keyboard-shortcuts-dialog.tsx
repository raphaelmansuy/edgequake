'use client';

import { KeyCombo } from '@/components/shared/kbd';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { SHORTCUT_SECTIONS, type ShortcutSection } from '@/lib/keyboard/catalog';
import { Keyboard } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface KeyboardShortcutsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCloseAutoFocus?: (event: Event) => void;
}

function Section({ section }: { section: ShortcutSection }) {
  const { t } = useTranslation();
  return (
    <section aria-labelledby={`kbd-${section.id}`} className="break-inside-avoid">
      <h3 id={`kbd-${section.id}`} className="mb-1.5 px-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
        {t(section.titleKey, section.fallback)}
      </h3>
      <ul>
        {section.entries.map((entry) => (
          <li key={entry.id} className="flex items-center justify-between gap-3 rounded-md px-2 py-1.5 hover:bg-muted/60">
            <span className="text-sm">{t(entry.labelKey, entry.fallback)}</span>
            <KeyCombo keys={entry.keys} kind={entry.kind} className="shrink-0" />
          </li>
        ))}
      </ul>
    </section>
  );
}

/**
 * Help dialog — rendered from the truthful shortcut catalog (SPEC-168), so it
 * can never list a binding that is not implemented.
 */
export function KeyboardShortcutsDialog({ open, onOpenChange, onCloseAutoFocus }: KeyboardShortcutsDialogProps) {
  const { t } = useTranslation();

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent onCloseAutoFocus={onCloseAutoFocus} className="max-h-[85dvh] overflow-y-auto sm:max-w-3xl" data-testid="keyboard-shortcuts-dialog">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Keyboard className="h-5 w-5" aria-hidden="true" />
            {t('keyboard.title', 'Keyboard shortcuts')}
          </DialogTitle>
          <DialogDescription>
            {t('keyboard.subtitle', 'Move through EdgeQuake without leaving the keyboard.')}
          </DialogDescription>
        </DialogHeader>

        <div className="gap-x-8 gap-y-5 py-2 sm:columns-2">
          {SHORTCUT_SECTIONS.map((section) => (
            <div key={section.id} className="mb-5 break-inside-avoid">
              <Section section={section} />
            </div>
          ))}
        </div>

        <p className="border-t pt-3 text-xs text-muted-foreground">
          {t('keyboard.footer', 'Single keys are ignored while you type or when a dialog is open.')}
        </p>
      </DialogContent>
    </Dialog>
  );
}

export default KeyboardShortcutsDialog;

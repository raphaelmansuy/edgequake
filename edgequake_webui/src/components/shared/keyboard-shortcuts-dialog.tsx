'use client';

import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog';
import { KEYBOARD_SHORTCUTS, type ShortcutDefinition } from '@/hooks/use-keyboard-shortcuts';
import { Keyboard } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface KeyboardShortcutsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

function ShortcutKey({ children }: { children: React.ReactNode }) {
  return (
    <kbd className="inline-flex items-center justify-center px-2 py-1 text-xs font-semibold text-foreground bg-muted border border-border rounded shadow-sm min-w-[24px]">
      {children}
    </kbd>
  );
}

function ShortcutRow({ shortcut }: { shortcut: ShortcutDefinition }) {
  return (
    <div className="flex items-center justify-between py-2 px-3 rounded hover:bg-muted/50 transition-colors">
      <span className="text-sm text-muted-foreground">{shortcut.description}</span>
      <div className="flex items-center gap-1">
        {shortcut.label.split(' + ').map((key, i) => (
          <span key={i} className="flex items-center">
            {i > 0 && <span className="text-muted-foreground mx-0.5">+</span>}
            <ShortcutKey>{key.replace('⌘/Ctrl', '⌘')}</ShortcutKey>
          </span>
        ))}
      </div>
    </div>
  );
}

/**
 * Dialog displaying all available keyboard shortcuts
 */
export function KeyboardShortcutsDialog({ open, onOpenChange }: KeyboardShortcutsDialogProps) {
  const { t } = useTranslation();

  // Group shortcuts by category
  const navigationShortcuts = KEYBOARD_SHORTCUTS.filter((s) => 
    s.description.toLowerCase().includes('go to') || s.description.toLowerCase().includes('navigation')
  );
  
  const actionShortcuts = KEYBOARD_SHORTCUTS.filter((s) => 
    !s.description.toLowerCase().includes('go to') && 
    !s.description.toLowerCase().includes('navigation')
  );

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Keyboard className="h-5 w-5" />
            {t('common.keyboardShortcuts', 'Keyboard Shortcuts')}
          </DialogTitle>
          <DialogDescription>
            {t('common.keyboardShortcutsDesc', 'Use these shortcuts to navigate quickly')}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-6 py-4">
          {/* Navigation shortcuts */}
          <div>
            <h4 className="text-sm font-medium text-foreground mb-2 px-3">
              {t('common.navigation', 'Navigation')}
            </h4>
            <div className="space-y-1">
              {navigationShortcuts.map((shortcut) => (
                <ShortcutRow key={shortcut.key} shortcut={shortcut} />
              ))}
            </div>
          </div>

          {/* Action shortcuts */}
          <div>
            <h4 className="text-sm font-medium text-foreground mb-2 px-3">
              {t('common.actions', 'Actions')}
            </h4>
            <div className="space-y-1">
              {actionShortcuts.map((shortcut) => (
                <ShortcutRow key={shortcut.key} shortcut={shortcut} />
              ))}
            </div>
          </div>
        </div>

        <div className="pt-2 border-t text-xs text-muted-foreground text-center">
          {t('common.pressEscToClose', 'Press Esc to close')}
        </div>
      </DialogContent>
    </Dialog>
  );
}

export default KeyboardShortcutsDialog;

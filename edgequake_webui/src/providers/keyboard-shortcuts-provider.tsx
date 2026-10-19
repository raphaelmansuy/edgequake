'use client';

import { ChordHint } from '@/components/shared/chord-hint';
import { CommandPalette } from '@/components/shared/command-palette';
import { KeyboardShortcutsDialog } from '@/components/shared/keyboard-shortcuts-dialog';
import { useKeyboardShortcuts } from '@/hooks/use-keyboard-shortcuts';
import { createContext, useContext, type ReactNode } from 'react';

interface KeyboardShortcutsContextType {
  helpOpen: boolean;
  openHelp: () => void;
  closeHelp: () => void;
  searchOpen: boolean;
  openSearch: () => void;
  closeSearch: () => void;
  /** Restores focus to the opener (Radix has no Trigger for hotkey-opened dialogs). */
  onCloseAutoFocus: (event: Event) => void;
}

const KeyboardShortcutsContext = createContext<KeyboardShortcutsContextType | null>(null);

export function useKeyboardShortcutsContext() {
  const context = useContext(KeyboardShortcutsContext);
  if (!context) {
    throw new Error('useKeyboardShortcutsContext must be used within a KeyboardShortcutsProvider');
  }
  return context;
}

interface KeyboardShortcutsProviderProps {
  children: ReactNode;
}

/**
 * Single mount for the global keyboard layer, ⌘K palette, help dialog and the
 * `g` chord hint (SPEC-155 F-155-S01/S02, SPEC-168).
 */
export function KeyboardShortcutsProvider({ children }: KeyboardShortcutsProviderProps) {
  const {
    helpOpen,
    setHelpOpen,
    openHelp,
    closeHelp,
    searchOpen,
    openSearch,
    closeSearch,
    chordArmed,
    onCloseAutoFocus,
  } = useKeyboardShortcuts();

  return (
    <KeyboardShortcutsContext.Provider
      value={{ helpOpen, openHelp, closeHelp, searchOpen, openSearch, closeSearch, onCloseAutoFocus }}
    >
      {children}
      <KeyboardShortcutsDialog open={helpOpen} onOpenChange={setHelpOpen} onCloseAutoFocus={onCloseAutoFocus} />
      <CommandPalette />
      <ChordHint armed={chordArmed} />
    </KeyboardShortcutsContext.Provider>
  );
}

export default KeyboardShortcutsProvider;

'use client';

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
 * Provider component that enables global keyboard shortcuts and manages the help dialog
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
  } = useKeyboardShortcuts();

  return (
    <KeyboardShortcutsContext.Provider
      value={{
        helpOpen,
        openHelp,
        closeHelp,
        searchOpen,
        openSearch,
        closeSearch,
      }}
    >
      {children}
      <KeyboardShortcutsDialog open={helpOpen} onOpenChange={setHelpOpen} />
    </KeyboardShortcutsContext.Provider>
  );
}

export default KeyboardShortcutsProvider;

"use client";

/**
 * @module use-keyboard-shortcuts
 * @description Hook for keyboard shortcut management.
 * Provides global keyboard navigation and accessibility.
 *
 * @implements FEAT0630 - Keyboard shortcut system
 * @implements FEAT0631 - Shortcut help modal
 * @implements FEAT0632 - Accessibility keyboard navigation
 *
 * @enforces BR0619 - Shortcuts do not conflict with browser defaults
 * @enforces BR0620 - Shortcuts visible in help dialog
 */

import { useRouter } from "next/navigation";
import { useCallback, useEffect, useState } from "react";

export interface ShortcutDefinition {
  key: string;
  label: string;
  description: string;
  modifiers?: ("meta" | "ctrl" | "shift" | "alt")[];
}

/**
 * Comprehensive list of all keyboard shortcuts in EdgeQuake WebUI
 */
export const KEYBOARD_SHORTCUTS: ShortcutDefinition[] = [
  {
    key: "g",
    label: "⌘/Ctrl + G",
    description: "Go to Knowledge Graph",
    modifiers: ["meta"],
  },
  {
    key: "d",
    label: "⌘/Ctrl + D",
    description: "Go to Documents",
    modifiers: ["meta"],
  },
  {
    key: "Q",
    label: "⌘/Ctrl + Shift + Q",
    description: "Go to Query",
    modifiers: ["meta", "shift"],
  },
  {
    key: ",",
    label: "⌘/Ctrl + ,",
    description: "Go to Settings",
    modifiers: ["meta"],
  },
  {
    key: "k",
    label: "⌘/Ctrl + K",
    description: "Open search/command palette",
    modifiers: ["meta"],
  },
  {
    key: "/",
    label: "⌘/Ctrl + /",
    description: "Show keyboard shortcuts",
    modifiers: ["meta"],
  },
  {
    key: "n",
    label: "⌘/Ctrl + N",
    description: "New conversation",
    modifiers: ["meta"],
  },
  {
    key: "h",
    label: "⌘/Ctrl + H",
    description: "Toggle history panel",
    modifiers: ["meta"],
  },
  {
    key: "e",
    label: "⌘/Ctrl + E",
    description: "Export conversation",
    modifiers: ["meta"],
  },
  { key: "?", label: "?", description: "Show keyboard shortcuts help" },
  { key: "Escape", label: "Esc", description: "Close dialogs and modals" },
];

interface UseKeyboardShortcutsOptions {
  onHelpOpen?: () => void;
  onSearchOpen?: () => void;
  onNewConversation?: () => void;
  onToggleHistory?: () => void;
  onExport?: () => void;
}

/**
 * Global keyboard shortcuts for EdgeQuake WebUI
 *
 * Shortcuts:
 * - Cmd/Ctrl + K: Open command palette / search
 * - Cmd/Ctrl + /: Show keyboard shortcuts
 * - Cmd/Ctrl + G: Go to Graph
 * - Cmd/Ctrl + D: Go to Documents
 * - Cmd/Ctrl + Shift + Q: Go to Query
 * - Cmd/Ctrl + ,: Go to Settings
 * - ?: Show keyboard shortcuts help
 * - Escape: Close modals/dialogs
 */
export function useKeyboardShortcuts(
  options: UseKeyboardShortcutsOptions = {}
) {
  const router = useRouter();
  const [helpOpen, setHelpOpen] = useState(false);
  const [searchOpen, setSearchOpen] = useState(false);

  const openHelp = useCallback(() => {
    setHelpOpen(true);
    options.onHelpOpen?.();
  }, [options]);

  const closeHelp = useCallback(() => {
    setHelpOpen(false);
  }, []);

  const openSearch = useCallback(() => {
    setSearchOpen(true);
    options.onSearchOpen?.();
  }, [options]);

  const closeSearch = useCallback(() => {
    setSearchOpen(false);
  }, []);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      // Skip if user is typing in an input (except for Escape)
      const target = e.target as HTMLElement;
      const isInputting =
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.isContentEditable;

      // Escape always works
      if (e.key === "Escape") {
        setHelpOpen(false);
        setSearchOpen(false);
        return;
      }

      // Skip other shortcuts if user is typing
      if (isInputting) {
        return;
      }

      const isMeta = e.metaKey || e.ctrlKey;

      // ? key: Show help
      if (e.key === "?" && !isMeta) {
        e.preventDefault();
        openHelp();
        return;
      }

      // Cmd/Ctrl + /: Show help
      if (isMeta && e.key === "/") {
        e.preventDefault();
        openHelp();
        return;
      }

      // Cmd/Ctrl + K: Open search
      if (isMeta && e.key === "k") {
        e.preventDefault();
        openSearch();
        return;
      }

      // Cmd/Ctrl + G: Go to Graph
      if (isMeta && e.key === "g") {
        e.preventDefault();
        router.push("/graph");
        return;
      }

      // Cmd/Ctrl + D: Go to Documents
      if (isMeta && e.key === "d") {
        e.preventDefault();
        router.push("/documents");
        return;
      }

      // Cmd/Ctrl + Shift + Q: Go to Query (use Shift to avoid quit conflict)
      if (isMeta && e.shiftKey && e.key === "Q") {
        e.preventDefault();
        router.push("/query");
        return;
      }

      // Cmd/Ctrl + ,: Go to Settings
      if (isMeta && e.key === ",") {
        e.preventDefault();
        router.push("/settings");
        return;
      }
    };

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [router, openHelp, openSearch]);

  return {
    helpOpen,
    setHelpOpen,
    openHelp,
    closeHelp,
    searchOpen,
    setSearchOpen,
    openSearch,
    closeSearch,
    shortcuts: KEYBOARD_SHORTCUTS,
  };
}

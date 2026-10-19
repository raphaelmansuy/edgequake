"use client";

/**
 * @module use-keyboard-shortcuts
 * @description Global keyboard layer (SPEC-168).
 *
 * Bindings (see `lib/keyboard/catalog.ts` for the truthful, rendered list):
 * - ⌘/Ctrl + K  toggle the command palette (works from inputs)
 * - ?           open the shortcut help
 * - [           collapse / expand the sidebar
 * - g then <x>  go to a destination (`lib/keyboard/destinations.ts`)
 *
 * WHY chords instead of ⌘G/⌘D: those combos are reserved by browsers
 * (find-next, bookmark) and never reached the page. Two-key sequences work in
 * every browser and do not collide with typing because they are suppressed in
 * editable controls and while a dialog/menu is open.
 *
 * @implements FEAT0630 - Keyboard shortcut system
 * @implements FEAT0631 - Shortcut help modal
 * @implements FEAT0632 - Accessibility keyboard navigation
 *
 * @enforces BR0619 - Shortcuts do not conflict with browser defaults
 * @enforces BR0620 - Shortcuts visible in help dialog
 */

import { CHORD_TIMEOUT_MS, createChordTracker } from "@/lib/keyboard/chords";
import { destinationForChord } from "@/lib/keyboard/destinations";
import { isEditableTarget, isOverlayOpen } from "@/lib/keyboard/dom";
import { toggleNavigation } from "@/lib/keyboard/nav-drawer";
import { createReturnFocus } from "@/lib/keyboard/return-focus";
import { useSettingsStore } from "@/stores/use-settings-store";
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useRef, useState } from "react";

const MODIFIER_KEYS = new Set(["Shift", "Control", "Alt", "Meta", "CapsLock"]);

function hasCommandModifier(e: KeyboardEvent): boolean {
  return e.metaKey || e.ctrlKey || e.altKey;
}

export function useKeyboardShortcuts() {
  const router = useRouter();
  const toggleSidebar = useSettingsStore((s) => s.toggleSidebar);
  const [helpOpen, setHelpOpen] = useState(false);
  const [searchOpen, setSearchOpen] = useState(false);
  const [chordArmed, setChordArmed] = useState(false);
  const tracker = useRef(createChordTracker());
  const disarmTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const [returnFocus] = useState(createReturnFocus);

  const openHelp = useCallback(() => {
    returnFocus.remember();
    setHelpOpen(true);
  }, [returnFocus]);
  const closeHelp = useCallback(() => setHelpOpen(false), []);
  const openSearch = useCallback(() => {
    returnFocus.remember();
    setSearchOpen(true);
  }, [returnFocus]);
  const closeSearch = useCallback(() => setSearchOpen(false), []);

  const disarm = useCallback(() => {
    tracker.current.reset();
    if (disarmTimer.current) clearTimeout(disarmTimer.current);
    disarmTimer.current = null;
    setChordArmed(false);
  }, []);

  useEffect(() => {
    const onChord = (e: KeyboardEvent): boolean => {
      const outcome = tracker.current.press(e.key, e.timeStamp);
      if (outcome.kind === "armed") {
        setChordArmed(true);
        disarmTimer.current = setTimeout(disarm, CHORD_TIMEOUT_MS);
        return true;
      }
      if (outcome.kind === "complete") {
        disarm();
        const dest = destinationForChord(outcome.key);
        if (dest) {
          e.preventDefault();
          router.push(dest.href);
          return true;
        }
      }
      return false;
    };

    const handler = (e: KeyboardEvent) => {
      if (e.defaultPrevented || e.isComposing) return;
      if (MODIFIER_KEYS.has(e.key)) return; // pressing Shift mid-chord must not cancel it

      // ⌘/Ctrl + K — toggles the palette, even from inside inputs.
      if ((e.metaKey || e.ctrlKey) && !e.altKey && e.key.toLowerCase() === "k") {
        e.preventDefault();
        disarm();
        returnFocus.remember();
        setSearchOpen((open) => !open);
        return;
      }

      // Everything below is a bare key: never while typing, never over a modal.
      if (hasCommandModifier(e) || e.repeat) return;
      if (isEditableTarget(e.target) || isOverlayOpen()) {
        if (tracker.current.armed) disarm();
        return;
      }

      if (e.key === "?") {
        e.preventDefault();
        disarm();
        returnFocus.remember();
        setHelpOpen(true);
        return;
      }

      if (e.key === "[") {
        e.preventDefault();
        disarm();
        toggleNavigation(toggleSidebar);
        return;
      }

      if (e.key.length === 1 && onChord(e)) return;
      if (tracker.current.armed) disarm();
    };

    window.addEventListener("keydown", handler);
    return () => {
      window.removeEventListener("keydown", handler);
      if (disarmTimer.current) clearTimeout(disarmTimer.current);
    };
  }, [router, toggleSidebar, disarm, returnFocus]);

  return {
    helpOpen,
    setHelpOpen,
    openHelp,
    closeHelp,
    searchOpen,
    setSearchOpen,
    openSearch,
    closeSearch,
    chordArmed,
    onCloseAutoFocus: returnFocus.restore,
  };
}

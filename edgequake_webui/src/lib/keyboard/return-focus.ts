/**
 * SPEC-168 — return focus to where the user was after a global overlay closes.
 *
 * Radix modal dialogs only restore focus to a mounted <Trigger>. The palette and
 * help dialog are opened by hotkeys (no trigger), so keyboard users were dropped
 * onto <body>. Remember the opener, restore it on close.
 */
export interface ReturnFocus {
  /** Call right before opening; ignored if focus is already inside an overlay. */
  remember: (doc?: Document) => void;
  /** Pass as `onCloseAutoFocus`. */
  restore: (event: Event) => void;
}

export function createReturnFocus(): ReturnFocus {
  let target: HTMLElement | null = null;
  return {
    remember(doc = document) {
      const active = doc.activeElement;
      if (!(active instanceof HTMLElement) || active === doc.body) {
        target = null;
        return;
      }
      if (active.closest('[role="dialog"], [role="alertdialog"]')) return; // keep original opener
      target = active;
    },
    restore(event) {
      if (!target || !target.isConnected) return; // let Radix do its default
      event.preventDefault();
      target.focus();
      target = null;
    },
  };
}

/**
 * SPEC-168 — DOM guards shared by every global keyboard handler (DRY).
 */

/** True when keystrokes belong to a text-entry control, not to the app. */
export function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return true;
  if (target.isContentEditable) return true;
  const role = target.getAttribute("role");
  return role === "textbox" || role === "searchbox" || role === "combobox";
}

/**
 * True while a modal surface (dialog, menu, listbox popup) owns the keyboard.
 * Single-key shortcuts must not fire underneath it.
 */
export function isOverlayOpen(doc: Document = document): boolean {
  return (
    doc.querySelector(
      '[role="dialog"][data-state="open"], [role="alertdialog"][data-state="open"], [role="menu"][data-state="open"], [role="listbox"][data-state="open"]',
    ) !== null
  );
}

/** Platform modifier label: ⌘ on Apple platforms, Ctrl elsewhere. */
export function isApplePlatform(nav: Pick<Navigator, "platform" | "userAgent"> = navigator): boolean {
  return /Mac|iPhone|iPad|iPod/i.test(nav.platform || nav.userAgent);
}

/**
 * SSOT containment class tokens for dashboard chrome.
 *
 * Prefer wrap / shrink over overflow-hidden so controls stay fully visible
 * inside their parent border (SPEC-146 dropzone polish).
 */

/** Flex row that wraps and can shrink inside a parent shell. */
export const CONTAIN_ROW =
  'min-w-0 w-full max-w-full flex flex-wrap items-center gap-2';

/** Trigger / button that may sit in a flex row — overrides default w-fit. */
export const CONTAIN_TRIGGER = 'min-w-0 max-w-full';

/** Label above control on narrow/tablet; row from lg+ (inventory breakpoint). */
export const CONTAIN_STACK =
  'flex flex-col gap-2 lg:flex-row lg:items-center min-w-0 w-full max-w-full';

/** Parser+sibling controls: one column until lg, then select | action. */
export const CONTAIN_COMBO =
  'w-full basis-full min-w-0 max-w-full grid grid-cols-1 gap-2 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-center';

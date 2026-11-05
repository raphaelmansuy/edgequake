/**
 * SPEC-155 — Upload dropzone layout adapted to its parent panel size.
 *
 * - row: short, wide band (tools strip) → icon | text | settings on one line
 * - stack: narrow column, or a short band too narrow for one line → vertical stack
 * - hero: roomy panel → centered column with subtitle
 */

export type DropzoneFillLayout = "row" | "stack" | "hero";

/** Below this height (px), prefer a single horizontal band. */
export const DROPZONE_ROW_MAX_HEIGHT_PX = 120;
/**
 * A band holds icon, parser, extraction mode and vision controls on one line
 * (the controls shrink and truncate) only from this width (px). Narrower bands
 * stack instead of clipping.
 */
export const DROPZONE_ROW_MIN_WIDTH_PX = 700;
/** Below this width (px), prefer a vertical stack. */
export const DROPZONE_STACK_MAX_WIDTH_PX = 280;
/**
 * At or above this height, a non-busy fill zone centers an invitation
 * and docks parser settings on the bottom edge. Shorter panels keep a
 * compact prompt so the invitation cannot overflow the zone.
 */
export const DROPZONE_ROOMY_MIN_HEIGHT_PX = 320;
/**
 * Below this height the inline settings bar does not fit under the invitation, so the bar
 * collapses behind one "Upload settings" button. Measured: the invitation row is 56px and the
 * inline bar is 44px, so they need ~104px together; a 76px band silently clipped the selects,
 * which is how the upload controls disappeared on a normal screen with the preview panel open.
 */
export const DROPZONE_INLINE_BAR_MIN_HEIGHT_PX = 112;
/**
 * At or below this height the zone cannot hold two rows at all (a 56px invitation over a 28px
 * settings bar is 84px, and a docked band is routinely 65–76px). The band then becomes one row:
 * icon, truncated invitation, settings menu. Measured on a 1024-wide window with the preview
 * panel open: the column is 216×65 and two rows clipped 345px of content into 97.
 */
export const DROPZONE_COMPACT_BAND_MAX_HEIGHT_PX = 92;

/**
 * True when the inline settings bar must give way to the single-button form.
 *
 * A docked zone is not free to grow: the Library keeps the rest of the screen, so the upload
 * band can legitimately be short. Adapting to that is the difference between "compact" and
 * "controls you cannot see". Width is checked too, because a narrow column stacks the controls
 * and needs more height than a wide band does.
 */
export function shouldCollapseSettingsBar(opts: {
  /** Measured height of the dropzone box, in px. */
  height: number;
  /** True when the box is in the vertical stack layout. */
  stacked: boolean;
}): boolean {
  const floor = opts.stacked
    ? DROPZONE_INLINE_BAR_MIN_HEIGHT_PX + 44
    : DROPZONE_INLINE_BAR_MIN_HEIGHT_PX;
  return opts.height < floor;
}

export function resolveDropzoneFillLayout(
  width: number,
  height: number,
): DropzoneFillLayout {
  if (!(width > 0) || !(height > 0)) return "hero";
  if (height < DROPZONE_ROW_MAX_HEIGHT_PX) {
    return width >= DROPZONE_ROW_MIN_WIDTH_PX ? "row" : "stack";
  }
  if (width < DROPZONE_STACK_MAX_WIDTH_PX) return "stack";
  return "hero";
}

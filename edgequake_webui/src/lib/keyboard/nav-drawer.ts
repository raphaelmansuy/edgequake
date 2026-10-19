/**
 * SPEC-168 — one rule for "show me the full navigation".
 *
 * ≥ lg the persisted sidebar toggles between rail and expanded. Below lg the
 * sidebar is always a rail (content keeps its width), so "expand" opens the
 * navigation drawer instead. Sidebar button, `[` key and palette action all
 * go through `toggleNavigation` so they can never disagree.
 */
export const NAV_DRAWER_EVENT = "eq:nav-drawer";
export const WIDE_VIEWPORT_QUERY = "(min-width: 1024px)";

export function isWideViewport(win: Pick<Window, "matchMedia"> = window): boolean {
  return win.matchMedia(WIDE_VIEWPORT_QUERY).matches;
}

export function requestNavDrawer(win: Pick<Window, "dispatchEvent"> = window): void {
  win.dispatchEvent(new CustomEvent(NAV_DRAWER_EVENT));
}

/** Wide: collapse/expand the sidebar. Narrow: open the drawer. */
export function toggleNavigation(toggleSidebar: () => void): void {
  if (isWideViewport()) toggleSidebar();
  else requestNavDrawer();
}

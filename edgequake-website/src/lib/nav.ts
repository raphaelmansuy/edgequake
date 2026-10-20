/**
 * Shared primary navigation (marketing header + Starlight docs header).
 * Keep labels short; trailing slashes match astro.config trailingSlash: "always".
 */
export type NavLink = {
  label: string;
  href: string;
  /** Prefix-match for aria-current (docs, tutorial). */
  match?: "exact" | "prefix";
};

export const SITE_NAV: NavLink[] = [
  { label: "Docs", href: "/docs/", match: "prefix" },
  { label: "Tutorial", href: "/tutorial/", match: "prefix" },
  { label: "Demo", href: "/demo/", match: "exact" },
  { label: "Ecosystem", href: "/ecosystem/", match: "exact" },
  { label: "Enterprise", href: "/enterprise/", match: "exact" },
  { label: "Collaborate", href: "/work-with-raphael/", match: "exact" },
  { label: "Contact", href: "/contact/", match: "exact" },
];

export const CTA_GET_STARTED = {
  label: "Get Started",
  href: "/docs/getting-started/",
} as const;

export function isActivePath(pathname: string, link: NavLink): boolean {
  if (link.match === "prefix") {
    return pathname.startsWith(link.href);
  }
  return pathname === link.href || pathname === link.href.replace(/\/$/, "");
}

/**
 * SPEC-168 — single source of truth for app destinations.
 *
 * Consumed by the sidebar, the command palette, the `g` chord shortcuts and
 * the keyboard help dialog so a route is declared exactly once.
 */
import {
  Activity,
  BarChart2,
  BookOpen,
  FileText,
  FolderKanban,
  Home,
  MessageSquare,
  Network,
  Settings,
  Terminal,
  type LucideIcon,
} from "lucide-react";

export interface Destination {
  href: string;
  icon: LucideIcon;
  /** i18n key for the label (shared with the sidebar). */
  labelKey: string;
  /** English fallback when the key is missing. */
  fallback: string;
  /** Second key of the `g` chord (`g` then this key). */
  chord: string;
}

export interface DestinationGroup {
  id: "primary" | "knowledge" | "system";
  labelKey: string | null;
  label: string | null;
  items: readonly Destination[];
}

export const DESTINATION_GROUPS: readonly DestinationGroup[] = [
  {
    id: "primary",
    labelKey: null,
    label: null,
    items: [
      { href: "/", icon: Home, labelKey: "nav.dashboard", fallback: "Dashboard", chord: "h" },
      { href: "/documents", icon: FileText, labelKey: "nav.documents", fallback: "Documents", chord: "d" },
      { href: "/query", icon: MessageSquare, labelKey: "nav.query", fallback: "Query", chord: "q" },
      { href: "/graph", icon: Network, labelKey: "nav.graph", fallback: "Knowledge Graph", chord: "g" },
    ],
  },
  {
    id: "knowledge",
    labelKey: "nav.groupKnowledge",
    label: "Knowledge",
    items: [
      { href: "/knowledge", icon: BookOpen, labelKey: "nav.knowledge", fallback: "Knowledge", chord: "k" },
      { href: "/workspace", icon: FolderKanban, labelKey: "nav.workspace", fallback: "Workspace", chord: "w" },
    ],
  },
  {
    id: "system",
    labelKey: "nav.groupSystem",
    label: "System",
    items: [
      { href: "/pipeline", icon: Activity, labelKey: "nav.pipeline", fallback: "Pipeline", chord: "p" },
      { href: "/costs", icon: BarChart2, labelKey: "nav.costs", fallback: "Costs", chord: "c" },
      { href: "/api-explorer", icon: Terminal, labelKey: "nav.apiExplorer", fallback: "API Explorer", chord: "a" },
      { href: "/settings", icon: Settings, labelKey: "nav.settings", fallback: "Settings", chord: "s" },
    ],
  },
];

export const DESTINATIONS: readonly Destination[] = DESTINATION_GROUPS.flatMap(
  (g) => g.items,
);

/** Resolve the destination bound to the second key of a `g` chord. */
export function destinationForChord(key: string): Destination | undefined {
  const k = key.toLowerCase();
  return DESTINATIONS.find((d) => d.chord === k);
}

/** Active-route rule shared by the sidebar and breadcrumbs. */
export function isActiveDestination(pathname: string, href: string): boolean {
  return href === "/"
    ? pathname === "/"
    : pathname === href || pathname.startsWith(`${href}/`);
}

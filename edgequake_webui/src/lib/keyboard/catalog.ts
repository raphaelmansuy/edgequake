/**
 * SPEC-168 — truthful keyboard shortcut catalog.
 *
 * Every entry here is implemented by a real handler (global hook or a page
 * hook). The help dialog renders this list, so it can never advertise a binding
 * that does not exist. Navigation rows are derived from `destinations.ts`.
 */
import { CHORD_LEADER } from "./chords";
import { DESTINATIONS } from "./destinations";

/** `Mod` renders as ⌘ on Apple platforms and Ctrl elsewhere. */
export const MOD = "Mod";

export interface ShortcutEntry {
  id: string;
  /** `combo`: pressed together (⌘ + K). `sequence`: pressed one after another (G then D). */
  kind: "combo" | "sequence" | "single";
  keys: readonly string[];
  labelKey: string;
  fallback: string;
}

export interface ShortcutSection {
  id: "navigation" | "global" | "documents" | "query" | "graph";
  titleKey: string;
  fallback: string;
  entries: readonly ShortcutEntry[];
}

const navigationEntries: ShortcutEntry[] = DESTINATIONS.map((d) => ({
  id: `go-${d.chord}`,
  kind: "sequence" as const,
  keys: [CHORD_LEADER.toUpperCase(), d.chord.toUpperCase()],
  labelKey: d.labelKey,
  fallback: d.fallback,
}));

export const SHORTCUT_SECTIONS: readonly ShortcutSection[] = [
  {
    id: "navigation",
    titleKey: "keyboard.sections.navigation",
    fallback: "Go to",
    entries: navigationEntries,
  },
  {
    id: "global",
    titleKey: "keyboard.sections.global",
    fallback: "Everywhere",
    entries: [
      { id: "palette", kind: "combo", keys: [MOD, "K"], labelKey: "keyboard.palette", fallback: "Open command palette" },
      { id: "help", kind: "single", keys: ["?"], labelKey: "keyboard.help", fallback: "Show keyboard shortcuts" },
      { id: "sidebar", kind: "single", keys: ["["], labelKey: "keyboard.sidebar", fallback: "Collapse or expand sidebar" },
      { id: "escape", kind: "single", keys: ["Esc"], labelKey: "keyboard.escape", fallback: "Close dialog, menu or panel" },
    ],
  },
  {
    id: "documents",
    titleKey: "keyboard.sections.documents",
    fallback: "Documents",
    entries: [
      { id: "doc-refresh", kind: "single", keys: ["R"], labelKey: "keyboard.docRefresh", fallback: "Refresh list" },
      { id: "doc-select-all", kind: "combo", keys: [MOD, "A"], labelKey: "keyboard.docSelectAll", fallback: "Select all documents" },
    ],
  },
  {
    id: "query",
    titleKey: "keyboard.sections.query",
    fallback: "Query",
    entries: [
      { id: "q-focus", kind: "single", keys: ["/"], labelKey: "keyboard.queryFocus", fallback: "Focus the question box" },
      { id: "q-mention", kind: "single", keys: ["@"], labelKey: "keyboard.queryMention", fallback: "Scope to a document" },
    ],
  },
  {
    id: "graph",
    titleKey: "keyboard.sections.graph",
    fallback: "Knowledge graph",
    entries: [
      { id: "g-search", kind: "single", keys: ["/"], labelKey: "keyboard.graphSearch", fallback: "Search nodes" },
      { id: "g-move", kind: "single", keys: ["←", "↑", "↓", "→"], labelKey: "keyboard.graphMove", fallback: "Move between connected nodes" },
      { id: "g-open", kind: "single", keys: ["Enter"], labelKey: "keyboard.graphFocus", fallback: "Focus the selected node" },
      { id: "g-depth", kind: "single", keys: ["1", "2", "3"], labelKey: "keyboard.graphDepth", fallback: "Neighbourhood depth" },
      { id: "g-zoom", kind: "single", keys: ["+", "−", "0"], labelKey: "keyboard.graphZoom", fallback: "Zoom in, out, reset" },
      { id: "g-full", kind: "single", keys: ["F"], labelKey: "keyboard.graphFullscreen", fallback: "Toggle fullscreen" },
    ],
  },
];

/** Flat list — used by tests to assert uniqueness and by the palette hints. */
export const ALL_SHORTCUT_ENTRIES: readonly ShortcutEntry[] = SHORTCUT_SECTIONS.flatMap(
  (s) => s.entries,
);

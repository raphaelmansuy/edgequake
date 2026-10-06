/**
 * SPEC-155 — Documents docking workspace layout model.
 *
 * Binary split tree (VS Code / tmux style) over three zones:
 *   intake | runs | library
 *
 * Pure functions only — persistence and React live in the hook / UI layer.
 */

export type WorkspaceZoneId = "intake" | "runs" | "library";

export type SplitOrientation = "horizontal" | "vertical";

/** Dock target relative to the zone under the pointer. */
export type DockEdge = "left" | "right" | "top" | "bottom" | "center";

export type WorkspacePresetId =
  | "classic"
  | "library-left"
  | "library-right"
  | "library-center";

export type LayoutLeaf = {
  type: "leaf";
  zone: WorkspaceZoneId;
};

export type LayoutSplit = {
  type: "split";
  orientation: SplitOrientation;
  /** Relative sizes that sum to 100. */
  sizes: [number, number];
  children: [LayoutNode, LayoutNode];
};

export type LayoutNode = LayoutLeaf | LayoutSplit;

export type WorkspaceLayoutDocument = {
  version: 3;
  tree: LayoutNode;
  collapsed: Partial<Record<WorkspaceZoneId, boolean>>;
  maximized: WorkspaceZoneId | null;
  presetId: WorkspacePresetId | null;
};

export const WORKSPACE_ZONES: readonly WorkspaceZoneId[] = [
  "intake",
  "runs",
  "library",
] as const;

export const WORKSPACE_LAYOUT_STORAGE_KEY =
  "edgequake.documents.workspaceLayout.v3";

/** Legacy curated mode key — migrated once into a preset. */
export const LEGACY_LAYOUT_MODE_STORAGE_KEY =
  "edgequake.documents.pageLayoutMode";

export const ZONE_MIN_PX: Record<
  WorkspaceZoneId,
  { width: number; height: number }
> = {
  library: { width: 440, height: 224 },
  intake: { width: 200, height: 112 },
  runs: { width: 180, height: 96 },
};

/** Collapsed rail thickness (px) along the group axis. */
export const ZONE_RAIL_PX = 28;

export function minSizeForZone(
  zone: WorkspaceZoneId,
  orientation: SplitOrientation,
): number {
  const mins = ZONE_MIN_PX[zone];
  return orientation === "horizontal" ? mins.width : mins.height;
}

/**
 * Minimum size (px) a layout subtree needs along `orientation`
 * (the parent Group's axis). Nested splits accumulate correctly.
 */
export function minSizeForSubtree(
  node: LayoutNode,
  orientation: SplitOrientation,
  collapsed: Partial<Record<WorkspaceZoneId, boolean>> = {},
): number {
  if (node.type === "leaf") {
    if (collapsed[node.zone]) return ZONE_RAIL_PX;
    return minSizeForZone(node.zone, orientation);
  }
  const a = minSizeForSubtree(node.children[0], orientation, collapsed);
  const b = minSizeForSubtree(node.children[1], orientation, collapsed);
  if (node.orientation === orientation) {
    // Side-by-side on this axis: both need room + thin splitter.
    return a + b + 8;
  }
  // Stacked on the other axis: only the larger cross-section matters.
  return Math.max(a, b);
}

export const MOBILE_BREAKPOINT_PX = 768;

const DEFAULT_SIZES: [number, number] = [40, 60];
const EQUAL_SIZES: [number, number] = [50, 50];

function leaf(zone: WorkspaceZoneId): LayoutLeaf {
  return { type: "leaf", zone };
}

function split(
  orientation: SplitOrientation,
  a: LayoutNode,
  b: LayoutNode,
  sizes: [number, number] = EQUAL_SIZES,
): LayoutSplit {
  return {
    type: "split",
    orientation,
    sizes: normalizeSizes(sizes),
    children: [a, b],
  };
}

function normalizeSizes(sizes: [number, number]): [number, number] {
  const a = Number.isFinite(sizes[0]) ? Math.max(5, sizes[0]) : 50;
  const b = Number.isFinite(sizes[1]) ? Math.max(5, sizes[1]) : 50;
  const sum = a + b;
  if (sum <= 0) return [50, 50];
  return [(a / sum) * 100, (b / sum) * 100];
}

/** Classic: tools row on top, Library below. */
export function presetClassic(): LayoutNode {
  // Tools band stays compact but above intake min height; Library owns the page.
  return split(
    "vertical",
    split("horizontal", leaf("intake"), leaf("runs"), [48, 52]),
    leaf("library"),
    [16, 84],
  );
}

/** Library as tall left column; Upload above Runs on the right (tools stay narrow). */
export function presetLibraryLeft(): LayoutNode {
  return split(
    "horizontal",
    leaf("library"),
    split("vertical", leaf("intake"), leaf("runs"), [36, 64]),
    [74, 26],
  );
}

/** Library as tall right column; tools on the left (tools stay narrow). */
export function presetLibraryRight(): LayoutNode {
  return split(
    "horizontal",
    split("vertical", leaf("intake"), leaf("runs"), [36, 64]),
    leaf("library"),
    [26, 74],
  );
}

/** Library center, Intake left, Runs right (Focus-like). */
export function presetLibraryCenter(): LayoutNode {
  return split(
    "horizontal",
    leaf("intake"),
    split("horizontal", leaf("library"), leaf("runs"), [82, 18]),
    [16, 84],
  );
}

export const WORKSPACE_PRESETS: Record<
  WorkspacePresetId,
  { id: WorkspacePresetId; label: string; build: () => LayoutNode }
> = {
  classic: {
    id: "classic",
    label: "Classic (rows)",
    build: presetClassic,
  },
  "library-left": {
    id: "library-left",
    label: "Library left",
    build: presetLibraryLeft,
  },
  "library-right": {
    id: "library-right",
    label: "Library right",
    build: presetLibraryRight,
  },
  "library-center": {
    id: "library-center",
    label: "Library center",
    build: presetLibraryCenter,
  },
};

export const WORKSPACE_PRESET_ORDER: readonly WorkspacePresetId[] = [
  "classic",
  "library-left",
  "library-right",
  "library-center",
] as const;

/** Idle Classic: Runs starts as a rail so Upload owns the tools band. */
export const IDLE_RUNS_COLLAPSED: Partial<Record<WorkspaceZoneId, boolean>> = {
  runs: true,
};

export function defaultWorkspaceDocument(): WorkspaceLayoutDocument {
  return {
    version: 3,
    tree: presetClassic(),
    collapsed: { ...IDLE_RUNS_COLLAPSED },
    maximized: null,
    presetId: "classic",
  };
}

export function collectZones(node: LayoutNode): WorkspaceZoneId[] {
  if (node.type === "leaf") return [node.zone];
  return [...collectZones(node.children[0]), ...collectZones(node.children[1])];
}

export function hasAllZones(node: LayoutNode): boolean {
  const zones = new Set(collectZones(node));
  return WORKSPACE_ZONES.every((z) => zones.has(z)) && zones.size === 3;
}

/** Deep clone for immutable updates. */
export function cloneLayout(node: LayoutNode): LayoutNode {
  if (node.type === "leaf") return { type: "leaf", zone: node.zone };
  return {
    type: "split",
    orientation: node.orientation,
    sizes: [...node.sizes] as [number, number],
    children: [cloneLayout(node.children[0]), cloneLayout(node.children[1])],
  };
}

/**
 * Remove a zone leaf; promote the sibling when a split collapses to one child.
 * Returns null if the tree would be empty.
 */
export function removeZone(
  node: LayoutNode,
  zone: WorkspaceZoneId,
): LayoutNode | null {
  if (node.type === "leaf") {
    return node.zone === zone ? null : node;
  }
  const left = removeZone(node.children[0], zone);
  const right = removeZone(node.children[1], zone);
  if (!left && !right) return null;
  if (!left) return right;
  if (!right) return left;
  return {
    type: "split",
    orientation: node.orientation,
    sizes: node.sizes,
    children: [left, right],
  };
}

function replaceZone(
  node: LayoutNode,
  zone: WorkspaceZoneId,
  replacement: LayoutNode,
): LayoutNode {
  if (node.type === "leaf") {
    return node.zone === zone ? replacement : node;
  }
  return {
    type: "split",
    orientation: node.orientation,
    sizes: node.sizes,
    children: [
      replaceZone(node.children[0], zone, replacement),
      replaceZone(node.children[1], zone, replacement),
    ],
  };
}

export function swapZones(
  node: LayoutNode,
  a: WorkspaceZoneId,
  b: WorkspaceZoneId,
): LayoutNode {
  if (a === b) return cloneLayout(node);
  const mapped = (n: LayoutNode): LayoutNode => {
    if (n.type === "leaf") {
      if (n.zone === a) return leaf(b);
      if (n.zone === b) return leaf(a);
      return leaf(n.zone);
    }
    return {
      type: "split",
      orientation: n.orientation,
      sizes: [...n.sizes] as [number, number],
      children: [mapped(n.children[0]), mapped(n.children[1])],
    };
  };
  return mapped(node);
}

function splitForEdge(
  edge: Exclude<DockEdge, "center">,
  dragged: LayoutNode,
  target: LayoutNode,
): LayoutSplit {
  switch (edge) {
    case "left":
      return split("horizontal", dragged, target, DEFAULT_SIZES);
    case "right":
      return split("horizontal", target, dragged, [60, 40]);
    case "top":
      return split("vertical", dragged, target, DEFAULT_SIZES);
    case "bottom":
      return split("vertical", target, dragged, [60, 40]);
  }
}

/**
 * Dock `dragged` beside (or swap with) `target`.
 * No-op when zones are identical or either is missing from the tree.
 */
export function dockZone(
  tree: LayoutNode,
  dragged: WorkspaceZoneId,
  target: WorkspaceZoneId,
  edge: DockEdge,
): LayoutNode {
  if (dragged === target) return cloneLayout(tree);
  const zones = new Set(collectZones(tree));
  if (!zones.has(dragged) || !zones.has(target)) return cloneLayout(tree);

  if (edge === "center") {
    return swapZones(tree, dragged, target);
  }

  const without = removeZone(tree, dragged);
  if (!without) return cloneLayout(tree);

  const replacement = splitForEdge(edge, leaf(dragged), leaf(target));
  return replaceZone(without, target, replacement);
}

export function updateSplitSizes(
  tree: LayoutNode,
  path: number[],
  sizes: [number, number],
): LayoutNode {
  if (path.length === 0) {
    if (tree.type !== "split") return tree;
    return { ...tree, sizes: normalizeSizes(sizes) };
  }
  if (tree.type !== "split") return tree;
  const [head, ...rest] = path;
  const idx = head === 1 ? 1 : 0;
  const nextChildren = [...tree.children] as [LayoutNode, LayoutNode];
  nextChildren[idx] = updateSplitSizes(tree.children[idx], rest, sizes);
  return { ...tree, children: nextChildren };
}

export function sanitizeLayout(raw: unknown): WorkspaceLayoutDocument {
  const fallback = defaultWorkspaceDocument();
  if (!raw || typeof raw !== "object") return fallback;
  const doc = raw as Partial<WorkspaceLayoutDocument>;
  if (doc.version !== 3) return fallback;
  if (!doc.tree || !isValidTree(doc.tree)) return fallback;
  const collapsed: Partial<Record<WorkspaceZoneId, boolean>> = {};
  if (doc.collapsed && typeof doc.collapsed === "object") {
    for (const z of WORKSPACE_ZONES) {
      if (typeof doc.collapsed[z] === "boolean") {
        collapsed[z] = doc.collapsed[z];
      }
    }
  }
  const maximized =
    doc.maximized && WORKSPACE_ZONES.includes(doc.maximized)
      ? doc.maximized
      : null;
  const presetId =
    doc.presetId && doc.presetId in WORKSPACE_PRESETS ? doc.presetId : null;
  return {
    version: 3,
    tree: cloneLayout(doc.tree),
    collapsed,
    maximized,
    presetId,
  };
}

function isValidTree(node: unknown): node is LayoutNode {
  if (!node || typeof node !== "object") return false;
  const n = node as LayoutNode;
  if (n.type === "leaf") {
    return WORKSPACE_ZONES.includes(n.zone);
  }
  if (n.type === "split") {
    if (n.orientation !== "horizontal" && n.orientation !== "vertical") {
      return false;
    }
    if (!Array.isArray(n.sizes) || n.sizes.length !== 2) return false;
    if (!Array.isArray(n.children) || n.children.length !== 2) return false;
    if (!isValidTree(n.children[0]) || !isValidTree(n.children[1])) {
      return false;
    }
    return hasAllZones(n) || collectZones(n).length >= 1;
  }
  return false;
}

/** Full-document sanitize: tree must contain each zone exactly once. */
export function sanitizeWorkspaceDocument(
  raw: unknown,
): WorkspaceLayoutDocument {
  const doc = sanitizeLayout(raw);
  if (!hasAllZones(doc.tree)) {
    return defaultWorkspaceDocument();
  }
  const zones = collectZones(doc.tree);
  if (new Set(zones).size !== zones.length) {
    return defaultWorkspaceDocument();
  }
  return doc;
}

export function serializeWorkspace(
  doc: WorkspaceLayoutDocument,
): string {
  return JSON.stringify(doc);
}

export function deserializeWorkspace(
  raw: string | null | undefined,
): WorkspaceLayoutDocument | null {
  if (!raw) return null;
  try {
    return sanitizeWorkspaceDocument(JSON.parse(raw));
  } catch {
    return null;
  }
}

/**
 * Map legacy Focus/Split/Stack storage into a v2 document.
 * focus → library-center, split → classic, stack → classic (stacked tools).
 */
export function migrateLegacyLayoutMode(
  mode: string | null | undefined,
): WorkspaceLayoutDocument {
  const base = defaultWorkspaceDocument();
  if (mode === "focus") {
    return {
      ...base,
      tree: presetLibraryCenter(),
      presetId: "library-center",
    };
  }
  if (mode === "split") {
    return { ...base, tree: presetClassic(), presetId: "classic" };
  }
  if (mode === "stack") {
    return {
      ...base,
      tree: split(
        "vertical",
        leaf("intake"),
        split("vertical", leaf("runs"), leaf("library"), [35, 65]),
        [22, 78],
      ),
      presetId: null,
    };
  }
  return base;
}

export function readWorkspaceLayout(
  storage?: Storage | null,
): WorkspaceLayoutDocument {
  try {
    const store =
      storage ?? (typeof localStorage !== "undefined" ? localStorage : null);
    if (!store) return defaultWorkspaceDocument();
    const raw = store.getItem(WORKSPACE_LAYOUT_STORAGE_KEY);
    if (raw) {
      const parsed = deserializeWorkspace(raw);
      if (parsed) return parsed;
    }
    const legacy = store.getItem(LEGACY_LAYOUT_MODE_STORAGE_KEY);
    if (legacy) {
      const migrated = migrateLegacyLayoutMode(legacy);
      writeWorkspaceLayout(migrated, store);
      return migrated;
    }
  } catch {
    // ignore
  }
  return defaultWorkspaceDocument();
}

export function writeWorkspaceLayout(
  doc: WorkspaceLayoutDocument,
  storage?: Storage | null,
): void {
  try {
    const store =
      storage ?? (typeof localStorage !== "undefined" ? localStorage : null);
    if (!store) return;
    store.setItem(WORKSPACE_LAYOUT_STORAGE_KEY, serializeWorkspace(doc));
  } catch {
    // ignore quota / private-mode
  }
}

export function applyPreset(
  presetId: WorkspacePresetId,
  options?: { runsIdle?: boolean },
): WorkspaceLayoutDocument {
  const preset = WORKSPACE_PRESETS[presetId];
  const runsIdle = options?.runsIdle ?? true;
  return {
    version: 3,
    tree: preset.build(),
    collapsed: runsIdle ? { ...IDLE_RUNS_COLLAPSED } : {},
    maximized: null,
    presetId,
  };
}

export function setZoneCollapsed(
  doc: WorkspaceLayoutDocument,
  zone: WorkspaceZoneId,
  collapsed: boolean,
): WorkspaceLayoutDocument {
  return {
    ...doc,
    collapsed: { ...doc.collapsed, [zone]: collapsed },
  };
}

export function setMaximized(
  doc: WorkspaceLayoutDocument,
  zone: WorkspaceZoneId | null,
): WorkspaceLayoutDocument {
  return { ...doc, maximized: zone };
}

/** Mobile fallback: vertical stack Intake → Runs → Library (no splitters). */
export function mobileStackTree(): LayoutNode {
  return split(
    "vertical",
    leaf("intake"),
    split("vertical", leaf("runs"), leaf("library"), [30, 70]),
    [18, 82],
  );
}

export function zoneLabel(zone: WorkspaceZoneId): string {
  switch (zone) {
    case "intake":
      return "Upload";
    case "runs":
      return "Runs";
    case "library":
      return "Library";
  }
}

export function edgeLabel(edge: DockEdge): string {
  switch (edge) {
    case "left":
      return "Left";
    case "right":
      return "Right";
    case "top":
      return "Top";
    case "bottom":
      return "Bottom";
    case "center":
      return "Swap";
  }
}

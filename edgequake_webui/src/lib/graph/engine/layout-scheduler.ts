/**
 * LayoutScheduler — keeps node positions in sync with the *selected* layout
 * as the graph grows (SPEC-155 W4 follow-up).
 *
 * WHY: `applyDelta` drops every brand-new node on a ring around the origin.
 * Without a follow-up layout pass, streamed batches stay on those rings and the
 * canvas shows concentric arcs regardless of the chosen algorithm (e.g.
 * ForceAtlas2). This scheduler re-runs the active layout whenever nodes (or,
 * for force layouts, edges) are added.
 */
import type Graph from "graphology";
import {
  applyLayoutToGraph,
  type GraphLayoutMode,
  type GraphLayoutType,
} from "@/lib/graph/layouts";

/**
 * Node attribute: `false` while a node only has its provisional ring position
 * (set by `applyDelta`), `true` once a layout pass positioned it. Nodes added
 * by other code paths (no attribute) are never re-seeded.
 */
export const PLACED_ATTR = "placed";

/** Layouts whose result depends on edges (and therefore on late-arriving edges). */
const TOPOLOGY_LAYOUTS: ReadonlySet<GraphLayoutType> = new Set([
  "force",
  "force-directed",
]);

/** Above this share of new elements a full pass is used instead of a light one. */
const FULL_PASS_CHANGE_RATIO = 0.2;
const DEFAULT_DEBOUNCE_MS = 150;

export interface LayoutChange {
  addedNodes: number;
  addedEdges: number;
}

export interface LayoutSchedulerDeps {
  graph: Graph;
  getLayout: () => GraphLayoutType;
  /** Usable canvas edge in px; lets force layouts space nodes for the real size. */
  getViewportPx?: () => number | undefined;
  /** Called after positions were mutated (typically `sigma.scheduleRefresh`). */
  onApplied: () => void;
  debounceMs?: number;
}

export function isTopologyLayout(layout: GraphLayoutType): boolean {
  return TOPOLOGY_LAYOUTS.has(layout);
}

/** Mark every node as positioned by a layout pass. */
export function markAllPlaced(graph: Graph): void {
  graph.forEachNode((id) => graph.setNodeAttribute(id, PLACED_ATTR, true));
}

/**
 * Move not-yet-placed nodes next to the centroid of their placed neighbours so
 * an incremental force pass starts from a sensible spot instead of the ring.
 * Returns the number of nodes that were re-seeded.
 */
export function seedUnplacedNodes(graph: Graph): number {
  let seeded = 0;
  graph.forEachNode((id, attrs) => {
    if (attrs[PLACED_ATTR] !== false) return;

    let sx = 0;
    let sy = 0;
    let count = 0;
    graph.forEachNeighbor(id, (neighbor, nAttrs) => {
      if (nAttrs[PLACED_ATTR] === false) return;
      sx += nAttrs.x as number;
      sy += nAttrs.y as number;
      count += 1;
    });
    if (count === 0) return;

    // Deterministic jitter (hash of id) keeps siblings from stacking exactly.
    let h = 0;
    for (let i = 0; i < id.length; i++) h = (h * 31 + id.charCodeAt(i)) >>> 0;
    const angle = ((h % 360) * Math.PI) / 180;
    const jitter = 10 + (h % 20);
    graph.setNodeAttribute(id, "x", sx / count + Math.cos(angle) * jitter);
    graph.setNodeAttribute(id, "y", sy / count + Math.sin(angle) * jitter);
    seeded += 1;
  });
  return seeded;
}

export class LayoutScheduler {
  private timer: ReturnType<typeof setTimeout> | null = null;
  private pendingNodes = 0;
  private pendingEdges = 0;

  constructor(private readonly deps: LayoutSchedulerDeps) {}

  /**
   * Report a graph mutation. `orderBefore` is the node count before the delta:
   * an empty graph is laid out synchronously (no flash of ring positions).
   */
  notify(change: LayoutChange, orderBefore: number): void {
    const layout = this.deps.getLayout();
    const relevant =
      change.addedNodes > 0 ||
      (change.addedEdges > 0 && isTopologyLayout(layout));
    if (!relevant) return;

    if (orderBefore === 0 && change.addedNodes > 0) {
      this.cancel();
      this.run("initial");
      return;
    }

    this.pendingNodes += change.addedNodes;
    this.pendingEdges += change.addedEdges;
    if (this.timer) clearTimeout(this.timer);
    this.timer = setTimeout(() => this.flush(), this.deps.debounceMs ?? DEFAULT_DEBOUNCE_MS);
  }

  /** Run pending work immediately (also used by tests). */
  flush(): void {
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = null;
    }
    const { graph } = this.deps;
    const total = graph.order + graph.size;
    const ratio = total > 0 ? (this.pendingNodes + this.pendingEdges) / total : 1;
    this.pendingNodes = 0;
    this.pendingEdges = 0;
    if (graph.order === 0) return;
    this.run(ratio > FULL_PASS_CHANGE_RATIO ? "initial" : "streaming");
  }

  cancel(): void {
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = null;
    }
    this.pendingNodes = 0;
    this.pendingEdges = 0;
  }

  private run(mode: GraphLayoutMode): void {
    const { graph, getLayout, onApplied } = this.deps;
    if (graph.order === 0) return;
    const layout = getLayout();
    const frozen = new Map<string, { x: number; y: number }>();
    if (mode === "streaming") {
      graph.forEachNode((id, attrs) => {
        if (attrs[PLACED_ATTR] === true) {
          frozen.set(id, { x: attrs.x as number, y: attrs.y as number });
        }
      });
    }
    try {
      if (isTopologyLayout(layout)) seedUnplacedNodes(graph);
      applyLayoutToGraph(graph, layout, mode, {
        viewportPx: this.deps.getViewportPx?.(),
      });
      if (mode === "streaming") {
        for (const [id, pos] of frozen) {
          if (!graph.hasNode(id)) continue;
          graph.setNodeAttribute(id, "x", pos.x);
          graph.setNodeAttribute(id, "y", pos.y);
        }
      }
      markAllPlaced(graph);
      onApplied();
    } catch (error) {
      console.warn("[GraphEngine] Layout pass failed:", error);
    }
  }
}

import { describe, expect, it } from "vitest";
import { MultiGraph } from "graphology";
import type { GraphEdge, GraphNode } from "@/types";
import {
  applyDelta,
  diffToDelta,
  resolveEdgeKey,
} from "./apply-delta";

const ctx = {
  borderColor: "#fff",
  edgeColor: "#999",
  nodeSize: 10,
  showEdgeLabels: false,
  getNodeColor: () => "#64748b",
};

function node(id: string, label = id): GraphNode {
  return { id, label, node_type: "PERSON", degree: 1 };
}

function edge(
  id: string,
  source: string,
  target: string,
  relationship_type = "RELATED_TO",
): GraphEdge {
  return {
    id,
    source,
    target,
    relationship_type,
    weight: 1,
    source_ids: [],
    created_at: "2026-01-01T00:00:00Z",
  };
}

describe("applyDelta", () => {
  it("upserts nodes and edges on a MultiGraph without dropping parallels", () => {
    const graph = new MultiGraph({ type: "directed" });
    applyDelta(
      graph,
      {
        upsertNodes: [node("a"), node("b")],
        upsertEdges: [
          edge("e1", "a", "b", "WORKS_WITH"),
          edge("e2", "a", "b", "REPORTS_TO"),
        ],
      },
      ctx,
    );
    expect(graph.order).toBe(2);
    expect(graph.size).toBe(2);
    expect(graph.hasEdge("e1")).toBe(true);
    expect(graph.hasEdge("e2")).toBe(true);
  });

  it("preserves positions when upserting existing nodes", () => {
    const graph = new MultiGraph({ type: "directed" });
    applyDelta(graph, { upsertNodes: [node("a")] }, ctx);
    graph.setNodeAttribute("a", "x", 42);
    graph.setNodeAttribute("a", "y", 7);
    applyDelta(graph, { upsertNodes: [node("a", "A2")] }, ctx);
    expect(graph.getNodeAttribute("a", "x")).toBe(42);
    expect(graph.getNodeAttribute("a", "y")).toBe(7);
    expect(graph.getNodeAttribute("a", "label")).toBe("A2");
  });

  it("removes nodes and edges by id", () => {
    const graph = new MultiGraph({ type: "directed" });
    applyDelta(
      graph,
      {
        upsertNodes: [node("a"), node("b")],
        upsertEdges: [edge("e1", "a", "b")],
      },
      ctx,
    );
    applyDelta(
      graph,
      { removeEdgeIds: ["e1"], removeNodeIds: ["b"] },
      ctx,
    );
    expect(graph.hasEdge("e1")).toBe(false);
    expect(graph.hasNode("b")).toBe(false);
    expect(graph.hasNode("a")).toBe(true);
  });

  it("diffToDelta removes stale ids and upserts desired set", () => {
    const graph = new MultiGraph({ type: "directed" });
    applyDelta(
      graph,
      {
        upsertNodes: [node("a"), node("b")],
        upsertEdges: [edge("e1", "a", "b")],
      },
      ctx,
    );
    const delta = diffToDelta(graph, [node("a")], []);
    expect(delta.removeNodeIds).toContain("b");
    expect(delta.removeEdgeIds).toContain("e1");
    expect(delta.upsertNodes).toHaveLength(1);
  });
});

describe("resolveEdgeKey", () => {
  it("prefers explicit edge id", () => {
    expect(
      resolveEdgeKey({
        id: "custom",
        source: "a",
        target: "b",
        relationship_type: "X",
      }),
    ).toBe("custom");
  });
});

describe("edge programs", () => {
  it("draws lone edges straight and bends only parallel / reverse edges", () => {
    const graph = new MultiGraph({ type: "directed" });
    applyDelta(
      graph,
      {
        upsertNodes: [node("a"), node("b"), node("c")],
        upsertEdges: [
          edge("e1", "a", "b"),
          edge("e2", "b", "a", "REVERSE"),
          edge("e3", "a", "c"),
        ],
      },
      ctx,
    );

    // @sigma/edge-curve renders nothing at curvature 0 → lone edges use "arrow".
    expect(graph.getEdgeAttribute("e3", "type")).toBe("arrow");
    expect(graph.hasEdgeAttribute("e3", "curvature")).toBe(false);
    expect(graph.getEdgeAttribute("e1", "type")).toBe("curvedArrow");
    expect(graph.getEdgeAttribute("e2", "type")).toBe("curvedArrow");

    // Antiparallel edges must land on OPPOSITE sides of the chord with EQUAL
    // bow. sigma offsets the control point by (-dy, dx) * curvature, so the
    // reversed edge flips direction on its own: equal stored magnitudes mean
    // equal visible bow, and the pair stays symmetric.
    const c1 = graph.getEdgeAttribute("e1", "curvature") as number;
    const c2 = graph.getEdgeAttribute("e2", "curvature") as number;
    expect(Math.abs(c1)).toBe(Math.abs(c2));
    expect(Math.sign(c1)).toBe(Math.sign(c2));
  });

  it("keeps a self-loop visible instead of collapsing to nothing", () => {
    const graph = new MultiGraph({ type: "directed" });
    applyDelta(
      graph,
      {
        upsertNodes: [node("a"), node("b")],
        upsertEdges: [edge("loop", "a", "a", "SELF"), edge("ab", "a", "b", "R")],
      },
      ctx,
    );

    // A straight program has no chord to draw for source === target, so the edge
    // would vanish. It must use the curved program with a non-zero bow.
    expect(graph.getEdgeAttribute("loop", "type")).toBe("curvedArrow");
    expect(graph.getEdgeAttribute("loop", "curvature")).toBeGreaterThan(0);
    // The unrelated lone edge must still be straight.
    expect(graph.getEdgeAttribute("ab", "type")).toBe("arrow");
  });

  it("does not restyle self-loops during the parallel pass", () => {
    const graph = new MultiGraph({ type: "directed" });
    applyDelta(
      graph,
      {
        upsertNodes: [node("a")],
        upsertEdges: [edge("loop", "a", "a", "SELF")],
      },
      ctx,
    );
    // applyDelta already ran refreshParallelCurvature; the self-loop must keep
    // its curvature rather than being flattened to a straight edge.
    expect(graph.getEdgeAttribute("loop", "type")).toBe("curvedArrow");
    expect(graph.getEdgeAttribute("loop", "curvature")).toBeGreaterThan(0);
  });

  it("marks new nodes as unplaced until a layout pass runs", () => {
    const graph = new MultiGraph();
    applyDelta(graph, { upsertNodes: [node("a")] }, ctx);
    expect(graph.getNodeAttribute("a", "placed")).toBe(false);
  });

  /**
   * Regression: the old rule was `curvature = 0.25 + i * 0.2`, which bent every
   * parallel edge further to the SAME side and grew without bound (8 parallels
   * reached 1.45 and looped back past their own endpoints).
   */
  it("fans parallel edges to both sides and saturates", () => {
    const graph = new MultiGraph({ type: "directed" });
    const parallels = Array.from({ length: 8 }, (_, i) =>
      edge(`p${i}`, "a", "b", "R"),
    );
    applyDelta(
      graph,
      { upsertNodes: [node("a"), node("b")], upsertEdges: parallels },
      ctx,
    );

    const values = parallels.map(
      (e) => graph.getEdgeAttribute(e.id, "curvature") as number,
    );
    expect(values.some((v) => v > 0)).toBe(true);
    expect(values.some((v) => v < 0)).toBe(true);
    for (const v of values) expect(Math.abs(v)).toBeLessThanOrEqual(0.72);
    // No edge may collapse to zero, or the curved program draws nothing.
    expect(values).not.toContain(0);
  });

  it("gives antiparallel edges equal bow magnitude", () => {
    const graph = new MultiGraph({ type: "directed" });
    applyDelta(
      graph,
      {
        upsertNodes: [node("a"), node("b")],
        upsertEdges: [edge("fwd", "a", "b"), edge("rev", "b", "a", "R")],
      },
      ctx,
    );
    const fwd = graph.getEdgeAttribute("fwd", "curvature") as number;
    const rev = graph.getEdgeAttribute("rev", "curvature") as number;
    expect(Math.abs(fwd)).toBe(Math.abs(rev));
  });

  it("keeps a lone edge straight and clears stale curvature", () => {
    const graph = new MultiGraph({ type: "directed" });
    applyDelta(
      graph,
      {
        upsertNodes: [node("a"), node("b")],
        upsertEdges: [edge("only", "a", "b")],
      },
      ctx,
    );
    expect(graph.getEdgeAttribute("only", "type")).toBe("arrow");
    expect(graph.hasEdgeAttribute("only", "curvature")).toBe(false);
  });

  it("paints deterministically regardless of edge insertion order", () => {
    const build = (order: string[]) => {
      const graph = new MultiGraph({ type: "directed" });
      applyDelta(
        graph,
        {
          upsertNodes: [node("a"), node("b")],
          upsertEdges: order.map((id) =>
            edge(id, id.startsWith("r") ? "b" : "a", id.startsWith("r") ? "a" : "b"),
          ),
        },
        ctx,
      );
      const byId: Record<string, unknown> = {};
      for (const id of order) byId[id] = graph.getEdgeAttribute(id, "curvature");
      return byId;
    };
    expect(build(["f1", "f2", "f3", "r1"])).toEqual(
      build(["r1", "f3", "f2", "f1"]),
    );
  });
});

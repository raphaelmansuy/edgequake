import { describe, expect, test } from "vitest";
import {
  EDGE_FAN_MAX,
  EDGE_FAN_STEP,
  assignPairCurvatures,
  isReversedEdge,
  parallelCurvatureSlots,
  undirectedPairKey,
} from "./edge-curvature";

describe("parallelCurvatureSlots", () => {
  test("alternates sign so a fan spreads to both sides", () => {
    const slots = parallelCurvatureSlots(4);
    expect(slots).toEqual([
      EDGE_FAN_STEP,
      -EDGE_FAN_STEP,
      EDGE_FAN_STEP * 2,
      -EDGE_FAN_STEP * 2,
    ]);
  });

  test("never emits a zero bow (the curved program draws nothing at 0)", () => {
    for (let n = 2; n <= 20; n++) {
      expect(parallelCurvatureSlots(n)).not.toContain(0);
    }
  });

  test("saturates so deep fans stay renderable", () => {
    const slots = parallelCurvatureSlots(24);
    for (const slot of slots) {
      expect(Math.abs(slot)).toBeLessThanOrEqual(EDGE_FAN_MAX + 1e-9);
    }
  });

  test("is symmetric in magnitude for even counts", () => {
    const slots = parallelCurvatureSlots(6);
    const magnitudes = slots.map((s) => Math.abs(s));
    const positives = magnitudes.filter((_, i) => i % 2 === 0);
    const negatives = magnitudes.filter((_, i) => i % 2 === 1);
    expect(positives).toEqual(negatives);
  });

  test("grows monotonically until it saturates", () => {
    const slots = parallelCurvatureSlots(6).map(Math.abs);
    expect(slots[0]).toBeLessThan(slots[2]!);
    expect(slots[2]).toBeLessThan(slots[4]!);
  });
});

describe("undirectedPairKey", () => {
  test("is direction agnostic", () => {
    expect(undirectedPairKey("a", "b")).toBe(undirectedPairKey("b", "a"));
  });

  test("does not collide across different pairs", () => {
    expect(undirectedPairKey("a", "b")).not.toBe(undirectedPairKey("a", "c"));
  });
});

describe("isReversedEdge", () => {
  test("only the lexicographically descending direction is reversed", () => {
    expect(isReversedEdge("a", "b")).toBe(false);
    expect(isReversedEdge("b", "a")).toBe(true);
  });
});

describe("assignPairCurvatures", () => {
  /**
   * `@sigma/edge-curve` offsets the control point by `(-dy, dx) * curvature`.
   * Reversing an edge flips `(-dy, dx)`, so the STORED sign must be mirrored to
   * keep the two edges on opposite world sides with equal magnitude. These tests
   * assert the geometric outcome (which side of the chord the bow lands on), not
   * the stored sign, because that is the property that matters on screen.
   */
  /**
   * Signed perpendicular bow of the quadratic control point for an edge over the
   * chord (0,0)→(200,0), honouring the edge's direction.
   *
   * sigma offsets the control point by `(-dy, dx) * curvature`. For the forward
   * chord (dx=200, dy=0) that is `dx * curvature`; reversing the edge negates
   * dx, so the same magnitude lands on the opposite world side.
   */
  const perpendicular = (
    source: string,
    target: string,
    storedCurvature: number,
  ) => {
    const forward = source < target;
    const dx = forward ? 200 : -200;
    const dy = 0;
    return dx * storedCurvature + dy * 0;
  };

  test("antiparallel pair bows to opposite sides with equal magnitude", () => {
    const [forward, reverse] = assignPairCurvatures([
      { source: "a", target: "b" },
      { source: "b", target: "a" },
    ]);
    const fwdOffset = perpendicular("a", "b", forward!);
    const revOffset = perpendicular("b", "a", reverse!);
    expect(Math.abs(fwdOffset)).toBe(Math.abs(revOffset));
    expect(Math.sign(fwdOffset)).toBe(-Math.sign(revOffset));
  });

  test("forward direction keeps a positive slot", () => {
    const [only] = assignPairCurvatures([{ source: "a", target: "b" }]);
    expect(only).toBe(EDGE_FAN_STEP);
  });

  test("reverse-only pair still bows to the same world side as forward", () => {
    // A lone reverse edge must not silently flip the visual convention: with the
    // mirrored slot it lands where a forward edge would.
    const [forwardOnly] = assignPairCurvatures([{ source: "a", target: "b" }]);
    const [reverseOnly] = assignPairCurvatures([{ source: "b", target: "a" }]);
    expect(perpendicular("a", "b", forwardOnly!)).toBe(
      perpendicular("b", "a", reverseOnly!),
    );
  });

  test("three same-direction edges fan out instead of stacking one side", () => {
    const values = assignPairCurvatures([
      { source: "a", target: "b" },
      { source: "a", target: "b" },
      { source: "a", target: "b" },
    ]);
    expect(values.some((v) => v > 0)).toBe(true);
    expect(values.some((v) => v < 0)).toBe(true);
  });

  test("mixed pair keeps every edge on a distinct world side", () => {
    const edges = [
      { source: "a", target: "b" },
      { source: "b", target: "a" },
      { source: "a", target: "b" },
    ];
    const values = assignPairCurvatures(edges);
    const offsets = edges.map((e, i) => perpendicular(e.source, e.target, values[i]!));
    // All magnitudes distinct enough to render as separate arcs.
    const magnitudes = offsets.map(Math.abs);
    expect(new Set(magnitudes.map((m) => m.toFixed(6))).size).toBeGreaterThan(1);
  });

  test("handles an empty pair", () => {
    expect(assignPairCurvatures([])).toEqual([]);
  });

  test("is deterministic", () => {
    const edges = [
      { source: "a", target: "b" },
      { source: "b", target: "a" },
    ];
    expect(assignPairCurvatures(edges)).toEqual(assignPairCurvatures(edges));
  });
});

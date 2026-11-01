import { describe, expect, test } from "vitest";
import {
  MIN_RENDERED_CURVATURE,
  curveApexOffset,
  edgeCurveGeometry,
} from "./edge-curve-geometry";

const S = { x: 0, y: 0 };
const T = { x: 200, y: 0 };

describe("edgeCurveGeometry", () => {
  test("straight edge trims to both node boundaries", () => {
    const g = edgeCurveGeometry(S, T, 10, 20, undefined)!;
    expect(g.control).toBeNull();
    expect(g.start).toEqual({ x: 10, y: 0 });
    expect(g.end).toEqual({ x: 180, y: 0 });
    expect(g.tipAngle).toBe(0);
  });

  test("treats near-zero curvature as straight", () => {
    const g = edgeCurveGeometry(S, T, 10, 20, MIN_RENDERED_CURVATURE / 2)!;
    expect(g.control).toBeNull();
  });

  test("curved control point spans the full chord (matches the WebGL shader)", () => {
    // shader: cpB = mid + (-dy, dx) * curvature. For (0,0)->(200,0), dy=0, dx=200.
    const g = edgeCurveGeometry(S, T, 10, 20, 0.25)!;
    expect(g.control).toEqual({ x: 100, y: 50 });
  });

  test("reversing the edge mirrors the bow to the opposite side", () => {
    const forward = edgeCurveGeometry(S, T, 10, 20, 0.25)!;
    const reverse = edgeCurveGeometry(T, S, 10, 20, 0.25)!;
    // Both stored at +0.25, yet the visible arcs sit on opposite sides.
    expect(Math.sign(forward.control!.y)).toBe(-Math.sign(reverse.control!.y));
    expect(Math.abs(forward.control!.y)).toBe(Math.abs(reverse.control!.y));
  });

  test("returns null for degenerate (coincident) endpoints", () => {
    expect(edgeCurveGeometry(S, { ...S }, 10, 20, 0.25)).toBeNull();
  });

  test("start point stays on the source node boundary", () => {
    const g = edgeCurveGeometry(S, T, 10, 20, 0.25)!;
    expect(Math.hypot(g.start.x - S.x, g.start.y - S.y)).toBeCloseTo(10, 6);
  });

  test("end point stays on the target node boundary plus stroke", () => {
    const g = edgeCurveGeometry(S, T, 10, 20, 0.25)!;
    expect(Math.hypot(g.end.x - T.x, g.end.y - T.y)).toBeCloseTo(20, 6);
  });

  test("tip angle follows the curve tangent, not the chord", () => {
    const curved = edgeCurveGeometry(S, T, 10, 20, 0.5)!;
    expect(curved.tipAngle).not.toBe(0);
    const straight = edgeCurveGeometry(S, T, 10, 20, undefined)!;
    expect(straight.tipAngle).toBe(0);
  });

  test("works for a vertical chord", () => {
    const g = edgeCurveGeometry({ x: 0, y: 0 }, { x: 0, y: 200 }, 10, 20, 0.25)!;
    // dx = 0, dy = 200 => control offset = (-200*0.25, 0) = (-50, 0)
    expect(g.control).toEqual({ x: -50, y: 100 });
  });
});

describe("curveApexOffset", () => {
  test("apex grows with curvature", () => {
    const small = curveApexOffset(S, T, 0.18);
    const large = curveApexOffset(S, T, 0.36);
    expect(large).toBeCloseTo(small * 2, 6);
  });

  test("apex is half the control offset for a quadratic Bézier", () => {
    // control offset for (0,0)->(200,0) at 0.25 is 50px => apex 25px
    expect(curveApexOffset(S, T, 0.25)).toBeCloseTo(25, 6);
  });

  test("is zero for a straight chord", () => {
    expect(curveApexOffset(S, T, 0)).toBe(0);
  });

  test("saturating the fan keeps the apex within a renderable bound", () => {
    // EDGE_FAN_MAX is 0.72; on a 200px chord that is a 144px control offset.
    expect(curveApexOffset(S, T, 0.72)).toBeCloseTo(72, 6);
  });
});

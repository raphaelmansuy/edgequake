/**
 * Geometry for the focus-edge overlay.
 *
 * WHY a separate pure module: the overlay redraws edges lifted above the node
 * layer so a selection's links stay visible through dense overlapping discs.
 * Its job is to trace the EXACT same curve the WebGL program drew — if the two
 * disagree, the edge visibly jumps the moment it is hovered.
 *
 * The WebGL program (`@sigma/edge-curve`) computes its quadratic control point as
 *
 *     cpB = mid(nodeCentres) + (-dy, dx) * curvature
 *
 * where `dx`/`dy` span the *node centres*, not the trimmed endpoints. The
 * previous overlay used trimmed endpoints AND an extra `* 0.5`, so overlay arcs
 * were both shallower and misplaced. These helpers encode the correct math once.
 */

export interface Point {
  x: number;
  y: number;
}

export interface EdgeCurveGeometry {
  /** Stroke start, trimmed to the source node boundary. */
  start: Point;
  /** Stroke end, trimmed to the target node boundary plus arrow thickness. */
  end: Point;
  /** Quadratic control point, or null for a straight edge. */
  control: Point | null;
  /** Direction the arrowhead must point (radians). */
  tipAngle: number;
}

/** Minimum |curvature| treated as a real bend (below this the arc is invisible). */
export const MIN_RENDERED_CURVATURE = 1e-4;

/**
 * Compute the trimmed curve between two node centres.
 *
 * @param source        Source node centre in viewport coordinates.
 * @param target        Target node centre in viewport coordinates.
 * @param sourceRadius  Source node radius in viewport pixels.
 * @param targetReach   Target node radius + stroke thickness.
 * @param curvature     Signed bow, matching the WebGL program.
 */
export function edgeCurveGeometry(
  source: Point,
  target: Point,
  sourceRadius: number,
  targetReach: number,
  curvature: number | undefined,
): EdgeCurveGeometry | null {
  const dx = target.x - source.x;
  const dy = target.y - source.y;
  const len = Math.hypot(dx, dy);
  if (len < 1) return null;

  const curved =
    typeof curvature === "number" && Math.abs(curvature) > MIN_RENDERED_CURVATURE;

  if (!curved) {
    const ux = dx / len;
    const uy = dy / len;
    return {
      start: { x: source.x + ux * sourceRadius, y: source.y + uy * sourceRadius },
      end: { x: target.x - ux * targetReach, y: target.y - uy * targetReach },
      control: null,
      tipAngle: Math.atan2(dy, dx),
    };
  }

  const curvatureValue = curvature as number;

  // Control point spans the full chord, exactly like the WebGL shader.
  const control: Point = {
    x: (source.x + target.x) / 2 - dy * curvatureValue,
    y: (source.y + target.y) / 2 + dx * curvatureValue,
  };

  // A quadratic Bézier leaves P0 toward the control point and arrives at P2 from
  // it. Trimming along those tangents keeps the stroke endpoints on the curve.
  const startAngle = Math.atan2(control.y - source.y, control.x - source.x);
  const endAngle = Math.atan2(target.y - control.y, target.x - control.x);

  return {
    start: {
      x: source.x + Math.cos(startAngle) * sourceRadius,
      y: source.y + Math.sin(startAngle) * sourceRadius,
    },
    end: {
      x: target.x - Math.cos(endAngle) * targetReach,
      y: target.y - Math.sin(endAngle) * targetReach,
    },
    control,
    tipAngle: endAngle,
  };
}

/**
 * Perpendicular bow (apex offset in px) of a curve produced by
 * `edgeCurveGeometry` — used by tests to prove overlay and WebGL agree.
 *
 * A quadratic Bézier's midpoint is the average of its control polygon, so its
 * apex sits half-way between the chord midpoint and the control point.
 */
export function curveApexOffset(
  source: Point,
  target: Point,
  curvature: number,
): number {
  const dx = target.x - source.x;
  const dy = target.y - source.y;
  const control: Point = {
    x: (source.x + target.x) / 2 - dy * curvature,
    y: (source.y + target.y) / 2 + dx * curvature,
  };
  const chordMid: Point = {
    x: (source.x + target.x) / 2,
    y: (source.y + target.y) / 2,
  };
  const apex: Point = {
    x: (source.x + 2 * control.x + target.x) / 4,
    y: (source.y + 2 * control.y + target.y) / 4,
  };
  return Math.hypot(apex.x - chordMid.x, apex.y - chordMid.y);
}

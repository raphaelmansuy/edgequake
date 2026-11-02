/**
 * Edge curvature allocation for parallel / antiparallel edges.
 *
 * WHY this module exists (measured defects it fixes):
 *
 * 1. ASYMMETRIC FAN. The previous rule was `curvature = BASE + i * STEP`
 *    (0.25, 0.45, 0.65, …). Because curvature is a signed *bow*, every parallel
 *    edge bent further to the SAME side, so N edges produced one fat arc plus a
 *    stack of progressively wilder arcs — never a balanced fan.
 *
 * 2. UNBOUNDED GROWTH. With 8 parallel edges the last one reached curvature
 *    1.45: a loop so deep it circled back past its own endpoints and read as an
 *    unrelated decorative curve. Curvature must saturate.
 *
 * 3. ANTIPARALLEL COLLISION. `@sigma/edge-curve` offsets the control point by
 *    `(-dy, dx) * curvature`. Reversing an edge flips `(-dy, dx)`, so the SAME
 *    curvature value already bows the two directions to OPPOSITE sides. The old
 *    code gave A→B 0.25 and B→A 0.45, which separated them but asymmetrically
 *    (50px vs 90px of bow for a 200px edge). Direction must be normalised before
 *    the sign is applied, so both sides of a pair bow equally.
 *
 * The allocation below is direction-aware: slots are symmetric and saturating,
 * then mirrored for edges that run against the canonical (lexicographic) pair
 * orientation.
 */

/**
 * Bow increment between consecutive fan slots.
 * 0.18 stays legible at typical zoom without edges merging into one band.
 */
export const EDGE_FAN_STEP = 0.18;

/**
 * Hard ceiling on |curvature|.
 *
 * WHY: a quadratic control point at `curvature * chordLength` means 0.72 bows a
 * 200px edge by ~144px at its apex — already a pronounced arc. Beyond that the
 * curve stops reading as "these two nodes are related" and starts reading as
 * decoration. Saturation also keeps dense hubs stable as more edges arrive.
 */
export const EDGE_FAN_MAX = 0.72;

/** Largest slot magnitude that still respects `EDGE_FAN_MAX`. */
const MAX_SLOT = Math.round(EDGE_FAN_MAX / EDGE_FAN_STEP);

/**
 * Bow values for `count` edges sharing one node pair, in slot order.
 *
 * Magnitudes follow 1, 1, 2, 2, 3, 3, … with alternating signs:
 *   n=2 → +1, -1
 *   n=3 → +1, -1, +2
 *   n=4 → +1, -1, +2, -2
 *
 * WHY that sequence rather than a symmetric spread around zero: the curved
 * program renders NOTHING at curvature 0, so a slot exactly at zero would make
 * an edge vanish. Pairing magnitudes on both sides avoids zero for every count
 * while keeping the fan centred on the straight chord.
 */
export function parallelCurvatureSlots(count: number): number[] {
  const slots: number[] = [];
  let magnitude = 1;
  for (let i = 0; i < count; i++) {
    const sign = i % 2 === 0 ? 1 : -1;
    slots.push(sign * Math.min(magnitude, MAX_SLOT) * EDGE_FAN_STEP);
    // Advance magnitude after each completed (+,-) pair: 1,1,2,2,3,3…
    if (i % 2 === 1) magnitude += 1;
  }
  return slots;
}

/**
 * Canonical pair key — undirected, so A→B and B→A share one fan.
 */
export function undirectedPairKey(source: string, target: string): string {
  return source < target ? `${source}::${target}` : `${target}::${source}`;
}

/**
 * True when the edge runs against the canonical pair orientation.
 *
 * The renderer already flips the bow direction when an edge is reversed, so
 * mirroring the value here is what makes both directions bow outward equally.
 */
export function isReversedEdge(source: string, target: string): boolean {
  return source > target;
}

/**
 * Assign a curvature to every edge of one undirected pair.
 *
 * `edges` must be in a deterministic order (callers sort by edge id) so the same
 * graph always paints the same way across renders.
 */
export function assignPairCurvatures<T extends { source: string; target: string }>(
  edges: readonly T[],
): number[] {
  const slots = parallelCurvatureSlots(edges.length);
  return edges.map((edge, index) => {
    const slot = slots[index] ?? 0;
    return isReversedEdge(edge.source, edge.target) ? -slot : slot;
  });
}

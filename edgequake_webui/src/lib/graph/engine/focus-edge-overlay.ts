/**
 * Focus-edge overlay — redraws lifted edges above the node WebGL layer.
 *
 * Sigma's edge `zIndex` only sorts within the edge canvas; edges can never paint
 * above nodes. This canvas (inserted before `labels`) mirrors hoverNodes for
 * edges so a selection's links stay visible through dense overlapping discs.
 */
import type Sigma from "sigma";
import { edgeCurveGeometry } from "./edge-curve-geometry";

export const FOCUS_EDGES_LAYER_ID = "focusEdges";

export interface FocusEdgeOverlayOptions {
  /** Whether this edge should be redrawn on the overlay. */
  isLifted: (edgeId: string) => boolean;
  /** Stroke colour for lifted edges (focus theme). */
  getColor: () => string;
}

export interface FocusEdgeOverlayHandle {
  unbind(): void;
}

function syncCanvasSize(
  canvas: HTMLCanvasElement,
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  dpr: number,
): void {
  const w = Math.max(1, Math.round(width * dpr));
  const h = Math.max(1, Math.round(height * dpr));
  if (canvas.width !== w || canvas.height !== h) {
    canvas.width = w;
    canvas.height = h;
    canvas.style.width = `${width}px`;
    canvas.style.height = `${height}px`;
  }
  // Reset transform each paint so repeated DPR scales do not compound.
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
}

function drawArrowHead(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  angle: number,
  size: number,
): void {
  ctx.beginPath();
  ctx.moveTo(x, y);
  ctx.lineTo(
    x - size * Math.cos(angle - Math.PI / 7),
    y - size * Math.sin(angle - Math.PI / 7),
  );
  ctx.lineTo(
    x - size * Math.cos(angle + Math.PI / 7),
    y - size * Math.sin(angle + Math.PI / 7),
  );
  ctx.closePath();
  ctx.fill();
}

/**
 * Create the focusEdges canvas and paint lifted edges after every Sigma render.
 * Returns an unbind that removes the listener and kills the layer.
 */
export function bindFocusEdgeOverlay(
  sigma: Sigma,
  options: FocusEdgeOverlayOptions,
): FocusEdgeOverlayHandle {
  const canvas = sigma.createCanvas(FOCUS_EDGES_LAYER_ID, {
    beforeLayer: "labels",
  });
  // Hit-testing stays on the mouse layer.
  canvas.style.pointerEvents = "none";
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    sigma.killLayer(FOCUS_EDGES_LAYER_ID);
    return { unbind: () => undefined };
  }

  let cleaned = false;

  const paint = () => {
    if (cleaned) return;
    const { width, height } = sigma.getDimensions();
    const dpr =
      typeof window !== "undefined" && window.devicePixelRatio
        ? window.devicePixelRatio
        : 1;
    syncCanvasSize(canvas, ctx, width, height, dpr);
    ctx.clearRect(0, 0, width, height);

    const graph = sigma.getGraph();
    const fallbackColor = options.getColor();

    for (const edge of graph.edges()) {
      if (!options.isLifted(edge)) continue;
      const display = sigma.getEdgeDisplayData(edge);
      if (!display || display.hidden) continue;

      const source = graph.source(edge);
      const target = graph.target(edge);
      const sourceData = sigma.getNodeDisplayData(source);
      const targetData = sigma.getNodeDisplayData(target);
      if (!sourceData || !targetData || sourceData.hidden || targetData.hidden) {
        continue;
      }

      // Display data x/y are framed-graph coords after Sigma's process step.
      const sVp = sigma.framedGraphToViewport({
        x: sourceData.x,
        y: sourceData.y,
      });
      const tVp = sigma.framedGraphToViewport({
        x: targetData.x,
        y: targetData.y,
      });

      const sourceSize = sigma.scaleSize(sourceData.size);
      const targetSize = sigma.scaleSize(targetData.size);
      const stroke =
        typeof display.color === "string" && display.color
          ? display.color
          : fallbackColor;
      const thickness = Math.max(
        2,
        sigma.scaleSize(typeof display.size === "number" ? display.size : 2),
      );

      const curvatureRaw = graph.getEdgeAttribute(edge, "curvature");
      const curvature =
        typeof curvatureRaw === "number" ? curvatureRaw : undefined;

      const geometry = edgeCurveGeometry(
        { x: sVp.x, y: sVp.y },
        { x: tVp.x, y: tVp.y },
        sourceSize,
        targetSize + thickness,
        curvature,
      );
      if (!geometry) continue;

      ctx.strokeStyle = stroke;
      ctx.fillStyle = stroke;
      ctx.lineWidth = thickness;
      ctx.lineCap = "round";
      ctx.lineJoin = "round";

      ctx.beginPath();
      ctx.moveTo(geometry.start.x, geometry.start.y);
      if (geometry.control) {
        ctx.quadraticCurveTo(
          geometry.control.x,
          geometry.control.y,
          geometry.end.x,
          geometry.end.y,
        );
      } else {
        ctx.lineTo(geometry.end.x, geometry.end.y);
      }
      ctx.stroke();
      drawArrowHead(
        ctx,
        geometry.end.x,
        geometry.end.y,
        geometry.tipAngle,
        Math.max(6, thickness * 2.2),
      );
    }
  };

  const onKill = () => unbind();
  sigma.on("afterRender", paint);
  sigma.on("kill", onKill);

  const unbind = () => {
    if (cleaned) return;
    cleaned = true;
    sigma.off("afterRender", paint);
    sigma.off("kill", onKill);
    try {
      sigma.killLayer(FOCUS_EDGES_LAYER_ID);
    } catch {
      // Layer may already be gone during sigma.kill().
    }
  };

  paint();

  return { unbind };
}

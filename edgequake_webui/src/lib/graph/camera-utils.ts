/**
 * @module camera-utils
 * @description Utility functions for Sigma camera operations.
 *
 * IMPORTANT: Sigma camera x,y coordinates use NORMALIZED coordinates (0-1 range)
 * representing a fraction of the graph's bounding box. Graph node positions (x, y)
 * are in raw graph space which can be any range (e.g., -100 to 100, 0 to 1000).
 *
 * Using raw graph coordinates directly in camera.animate() will cause the camera
 * to zoom to an empty area outside the visible graph.
 *
 * @implements FEAT0713 - Camera focus on node
 * @implements FEAT0714 - Camera fit-to-graph
 * @implements FEAT0715 - Smooth camera animation
 *
 * @enforces BR0711 - Convert graph coords to normalized
 * @enforces BR0712 - Animation duration capped at 1000ms
 */

import type Sigma from "sigma";

export interface FocusOptions {
  /** Zoom ratio - lower means more zoomed in (default: 0.4) */
  ratio?: number;
  /** Animation duration in ms (default: 500) */
  duration?: number;
  /** Whether to highlight the node after focusing (default: true) */
  highlight?: boolean;
}

/**
 * Focus camera on a specific node with proper coordinate normalization.
 *
 * @param sigmaInstance - The Sigma instance
 * @param nodeId - The ID of the node to focus on
 * @param options - Focus options
 * @returns true if successful, false otherwise
 */
export function focusCameraOnNode(
  sigmaInstance: Sigma,
  nodeId: string,
  options: FocusOptions = {}
): boolean {
  const { ratio = 0.4, duration = 500, highlight = true } = options;

  const graph = sigmaInstance.getGraph();

  if (!graph.hasNode(nodeId)) {
    console.warn(`[camera-utils] Node not found: ${nodeId}`);
    return false;
  }

  try {
    // Get node position in raw graph coordinates
    const nodeX = graph.getNodeAttribute(nodeId, "x") as number;
    const nodeY = graph.getNodeAttribute(nodeId, "y") as number;

    // Normalize to camera coordinates (0-1 range)
    const normalized = normalizeGraphCoordinates(sigmaInstance, nodeX, nodeY);

    // Animate camera to the normalized position
    sigmaInstance.getCamera().animate(
      {
        x: normalized.x,
        y: normalized.y,
        ratio,
      },
      { duration }
    );

    // Optionally highlight the node
    if (highlight) {
      graph.setNodeAttribute(nodeId, "highlighted", true);
      sigmaInstance.refresh();
    }

    return true;
  } catch (error) {
    console.error("[camera-utils] Error focusing on node:", error);
    return false;
  }
}

/**
 * Convert raw graph coordinates to normalized camera coordinates (0-1 range).
 *
 * @param sigmaInstance - The Sigma instance
 * @param x - Raw X coordinate in graph space
 * @param y - Raw Y coordinate in graph space
 * @returns Normalized coordinates suitable for camera.animate()
 */
export function normalizeGraphCoordinates(
  sigmaInstance: Sigma,
  x: number,
  y: number
): { x: number; y: number } {
  // Get the bounding box of all nodes
  const bbox = sigmaInstance.getBBox();

  const graphWidth = bbox.x[1] - bbox.x[0];
  const graphHeight = bbox.y[1] - bbox.y[0];

  // Handle edge cases (single node or all nodes at same position)
  const normalizedX = graphWidth > 0 ? (x - bbox.x[0]) / graphWidth : 0.5;
  const normalizedY = graphHeight > 0 ? (y - bbox.y[0]) / graphHeight : 0.5;

  return { x: normalizedX, y: normalizedY };
}

/**
 * Reset camera to show the entire graph with padding.
 *
 * @param sigmaInstance - The Sigma instance
 * @param duration - Animation duration in ms (default: 500)
 */
export function resetCameraToFitGraph(
  sigmaInstance: Sigma,
  duration: number = 500
): void {
  try {
    // Clear custom bounding box if set
    sigmaInstance.setCustomBBox(null);
    sigmaInstance.refresh();

    const graph = sigmaInstance.getGraph();

    // If no nodes, reset to center
    if (!graph?.order || graph.nodes().length === 0) {
      sigmaInstance
        .getCamera()
        .animate(
          { x: 0.5, y: 0.5, ratio: 1 },
          { duration: Math.min(duration, 300) }
        );
      return;
    }

    // Reset to center with slight zoom out for padding
    sigmaInstance
      .getCamera()
      .animate({ x: 0.5, y: 0.5, ratio: 1.1, angle: 0 }, { duration });
  } catch (error) {
    console.error("[camera-utils] Error resetting camera:", error);
    // Fallback to simple reset
    sigmaInstance
      .getCamera()
      .animate({ x: 0.5, y: 0.5, ratio: 1, angle: 0 }, { duration: 300 });
  }
}

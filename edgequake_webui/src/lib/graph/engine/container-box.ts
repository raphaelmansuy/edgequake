/**
 * Sigma's `resize()` throws while the container's layout box is empty.
 * The query companion pane hits that on the first frame, and again whenever
 * the pane is collapsed, before the resizable column has a width.
 */
export function sigmaContainerBox(container: {
  offsetWidth: number;
  offsetHeight: number;
}): { width: number; height: number } | null {
  const width = container.offsetWidth;
  const height = container.offsetHeight;
  if (width <= 0 || height <= 0) return null;
  return { width, height };
}

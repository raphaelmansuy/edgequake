/**
 * @module GraphRenderer
 * @description Thin React shell over long-lived GraphEngine (SPEC-155 W4).
 * Filter/theme/node churn must NOT tear down Sigma (F-155-G01).
 */
'use client';

import { ErrorState } from '@/components/shared/error-state';
import { useEntityTypeColors } from '@/hooks/use-entity-type-colors';
import { resolveEntityTypeColor } from '@/lib/graph/entity-type-colors';
import {
  useGraphEngine,
  type GraphFiltersState,
} from '@/lib/graph/engine';
import type { GraphLayoutType } from '@/lib/graph/layouts';
import { useGraphStore } from '@/stores/use-graph-store';
import { useSettingsStore } from '@/stores/use-settings-store';
import type { GraphEdge, GraphNode } from '@/types';
import { useTheme } from 'next-themes';
import { useCallback, useEffect, useRef } from 'react';
import { useRouter } from 'next/navigation';
import { Button } from '@/components/ui/button';

const NODE_SIZES: Record<string, number> = {
  small: 6,
  medium: 10,
  large: 14,
};

interface GraphRendererProps {
  nodes: GraphNode[];
  edges: GraphEdge[];
  /** Filter state applied via engine.setFilters (dim, not rebuild). */
  filters?: Partial<GraphFiltersState> & {
    types?: Set<string> | string[];
    relTypes?: Set<string> | string[];
  };
  onNodeClick?: (nodeId: string) => void;
  onNodeDoubleClick?: (nodeId: string) => void;
  onStageClick?: () => void;
  onNodeHover?: (nodeId: string | null) => void;
  onNodeRightClick?: (nodeId: string, x: number, y: number) => void;
  /** Node whose context menu is open (emphasised until dismissed). */
  contextTargetId?: string | null;
  /**
   * Standalone mode (SPEC-157 LAW-157-9): selection comes from this prop and
   * the renderer neither reads focus/ego state from, nor publishes sigma to,
   * the workspace graph store — so embedding it cannot disturb Graph Studio.
   */
  isolated?: { selectedNodeId: string | null };
  /** Replaces the default WebGL-failure overlay (e.g. an accessible list). */
  errorSlot?: React.ReactNode;
}

const ISOLATED_FOCUS: { mode: string; ids: string[]; depth?: number } = {
  mode: 'none',
  ids: [],
};

export function GraphRenderer({
  nodes,
  edges,
  filters,
  onNodeClick,
  onNodeDoubleClick,
  onStageClick,
  onNodeHover,
  onNodeRightClick,
  contextTargetId = null,
  isolated,
  errorSlot,
}: GraphRendererProps) {
  const setSigmaInstance = useGraphStore((s) => s.setSigmaInstance);
  const storeSelectedNodeId = useGraphStore((s) => s.selectedNodeId);
  const colorMode = useGraphStore((s) => s.colorMode);
  const storeEngineFocus = useGraphStore((s) => s.engineFocus);
  const storeEgoDepth = useGraphStore((s) => s.egoDepth);
  const selectedNodeId = isolated ? isolated.selectedNodeId : storeSelectedNodeId;
  const engineFocus = isolated ? ISOLATED_FOCUS : storeEngineFocus;
  const egoDepth = isolated ? 1 : storeEgoDepth;
  const isIsolated = Boolean(isolated);
  const { graphSettings } = useSettingsStore();
  const { resolvedTheme } = useTheme();
  const isDark = resolvedTheme === 'dark';
  const { colors: entityTypeColorOverrides } = useEntityTypeColors();
  const entityTypeColorOverridesRef = useRef(entityTypeColorOverrides);
  useEffect(() => {
    entityTypeColorOverridesRef.current = entityTypeColorOverrides;
  }, [entityTypeColorOverrides]);
  const router = useRouter();

  const getNodeColor = useCallback(
    (entityType: string | undefined) =>
      resolveEntityTypeColor(entityType, entityTypeColorOverridesRef.current),
    [],
  );

  const showLabels = graphSettings.showLabels ?? true;
  const showEdgeLabels = graphSettings.showEdgeLabels ?? false;
  const enableNodeDrag = graphSettings.enableNodeDrag ?? true;
  const highlightNeighbors = graphSettings.highlightNeighbors ?? true;
  const hideUnselectedEdges = graphSettings.hideUnselectedEdges ?? false;
  const nodeSize = NODE_SIZES[graphSettings.nodeSize] ?? NODE_SIZES.medium;
  const layout = (graphSettings.layout ?? 'force') as GraphLayoutType;

  const { engine, engineId, error, containerRef } = useGraphEngine({
    getNodeColor,
    isDark,
    showLabels,
    showEdgeLabels,
    enableNodeDrag,
    highlightNeighbors,
    hideUnselectedEdges,
    nodeSize,
    layout,
    colorMode,
    onNodeClick,
    onNodeDoubleClick,
    onStageClick,
    onNodeHover,
    onNodeRightClick,
  });

  // Publish sigma to store for zoom/minimap/keyboard (stable across filters)
  useEffect(() => {
    if (!engine || isIsolated) return;
    const sigma = engine.getSigma();
    setSigmaInstance(sigma);
    // E2E probe: only exposed when a test opts in via window.__EQ_E2E__.
    const probe = window as unknown as { __EQ_E2E__?: boolean; __eqSigma?: unknown };
    if (probe.__EQ_E2E__) probe.__eqSigma = sigma;
    return () => setSigmaInstance(null);
  }, [engine, setSigmaInstance, isIsolated]);

  // Data sync — applyDelta only (G01 / G05 MultiGraph inside engine).
  // The engine positions new nodes with the selected layout (LayoutScheduler),
  // including nodes/edges that stream in after the first batch.
  useEffect(() => {
    if (!engine) return;
    engine.syncData(nodes, edges);
  }, [engine, nodes, edges]);

  // Filters → dim layer (G02 debounce happens upstream; G06 time filter in pipeline)
  useEffect(() => {
    if (!engine || !filters) return;
    engine.setFilters(filters);
  }, [engine, filters]);

  // Ego depth: how many hops stay bright around the selected node
  useEffect(() => {
    engine?.setEgoDepth(egoDepth);
  }, [engine, egoDepth]);

  // Selection emphasize
  useEffect(() => {
    engine?.setSelected(selectedNodeId);
  }, [engine, selectedNodeId]);

  // Answer / ego / path focus from store (SPEC-155 W5/W6)
  useEffect(() => {
    if (!engine) return;
    engine.setFocus({
      mode: (engineFocus.mode as
        | "none"
        | "hover"
        | "select"
        | "ego"
        | "path"
        | "answer") || "none",
      ids: engineFocus.ids,
      depth: engineFocus.depth,
    });
  }, [engine, engineFocus]);

  // Theme without kill
  useEffect(() => {
    engine?.setTheme(undefined, isDark);
  }, [engine, isDark]);

  useEffect(() => {
    engine?.setContextTarget(contextTargetId);
  }, [engine, contextTargetId]);

  // Settings / color mode without kill
  useEffect(() => {
    engine?.updateOptions({
      showLabels,
      showEdgeLabels,
      enableNodeDrag,
      highlightNeighbors,
      hideUnselectedEdges,
      nodeSize,
      colorMode,
      getNodeColor,
      isDark,
      // Keep handlers fresh: the engine would otherwise keep creation-time closures.
      onNodeClick,
      onNodeDoubleClick,
      onStageClick,
      onNodeHover,
      onNodeRightClick,
    });
  }, [
    engine,
    onNodeClick,
    onNodeDoubleClick,
    onStageClick,
    onNodeHover,
    onNodeRightClick,
    showLabels,
    showEdgeLabels,
    enableNodeDrag,
    highlightNeighbors,
    hideUnselectedEdges,
    nodeSize,
    colorMode,
    getNodeColor,
    isDark,
  ]);

  // Selected layout changed (or engine created before settings hydrated):
  // re-run it and animate. Compares with the engine, the single source of truth.
  useEffect(() => {
    if (!engine || engine.getLayout() === layout) return;
    engine.setLayoutMode(layout);
  }, [engine, layout]);

  // Recolor on entity-type override change
  useEffect(() => {
    if (!engine || colorMode !== 'entity-type') return;
    const graph = engine.getGraph();
    graph.forEachNode((nodeId) => {
      const entityType = graph.getNodeAttribute(nodeId, 'entityType') as
        | string
        | undefined;
      graph.setNodeAttribute(nodeId, 'color', getNodeColor(entityType));
    });
    engine.getSigma()?.refresh();
  }, [engine, entityTypeColorOverrides, colorMode, getNodeColor]);

  return (
    <div
      ref={containerRef}
      className="absolute inset-0 rounded-lg bg-muted/20 outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background"
      data-graph-engine-id={engineId ?? undefined}
      data-graph-engine={engineId ? '1' : undefined}
      tabIndex={0}
      // Keyboard shortcuts act on the focused canvas, so a pointer press must focus it.
      onPointerDown={(e) => e.currentTarget.focus({ preventScroll: true })}
      role="application"
      aria-label="Knowledge graph canvas"
    >
      {error && errorSlot ? (
        <div className="absolute inset-0 z-10 bg-background">{errorSlot}</div>
      ) : error ? (
        <div className="absolute inset-0 z-10 flex items-center justify-center bg-background/95 p-4">
          <ErrorState
            title="Graph visualization unavailable"
            description="WebGL could not start in this browser. You can still browse entities as a table."
            className="max-w-md border-0 bg-transparent"
          />
          <div className="absolute bottom-8 flex gap-2">
            <Button
              type="button"
              variant="outline"
              onClick={() => router.push('/graph?view=table')}
            >
              Open as table
            </Button>
          </div>
        </div>
      ) : null}
    </div>
  );
}

export default GraphRenderer;

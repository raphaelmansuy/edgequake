/**
 * SPEC-157 W3 — Self-contained graph canvas for an answer's evidence.
 * Reuses the Sigma engine via GraphRenderer in `isolated` mode: no reads or
 * writes of the workspace graph store (LAW-157-9), selection is local.
 */
"use client";

import { GraphRenderer } from "@/components/graph/graph-renderer";
import type { GraphEdge, GraphNode } from "@/types/graph";
import type { ReactNode } from "react";

interface EmbeddedAnswerGraphProps {
  nodes: GraphNode[];
  edges: GraphEdge[];
  selectedNodeId: string | null;
  onSelect: (nodeId: string | null) => void;
  onExpand?: (nodeId: string) => void;
  /** Rendered instead of the canvas when WebGL cannot start. */
  fallback: ReactNode;
}

export function EmbeddedAnswerGraph({
  nodes,
  edges,
  selectedNodeId,
  onSelect,
  onExpand,
  fallback,
}: EmbeddedAnswerGraphProps) {
  return (
    <div className="relative h-full min-h-0" data-testid="companion-graph-canvas">
      <GraphRenderer
        nodes={nodes}
        edges={edges}
        isolated={{ selectedNodeId }}
        onNodeClick={onSelect}
        onNodeDoubleClick={onExpand}
        onStageClick={() => onSelect(null)}
        errorSlot={fallback}
      />
    </div>
  );
}

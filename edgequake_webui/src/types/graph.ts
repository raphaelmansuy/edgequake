/** Graph visualization and PDF parser backend types. */

export interface GraphNode {
  id: string;
  label: string;
  node_type: string;
  description?: string;
  degree?: number;
  properties?: Record<string, unknown>;
  created_at?: string;
  updated_at?: string;
}

export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  relationship_type: string;
  weight: number;
  description?: string;
  source_ids: string[];
  properties?: Record<string, unknown>;
  created_at: string;
}

export interface KnowledgeGraph {
  nodes: GraphNode[];
  edges: GraphEdge[];
  metadata: {
    node_count: number;
    edge_count: number;
    entity_types: string[];
    relationship_types: string[];
  };
  /** Whether the graph was truncated due to max_nodes limit */
  is_truncated?: boolean;
  /** Total node count in storage (before truncation) */
  total_nodes?: number;
  /** Total edge count in storage (before truncation) */
  total_edges?: number;
}

export type PdfParserBackend = "vision" | "edgeparse";
export type WorkspacePdfParserBackendUpdate = PdfParserBackend | "none";

/** Application settings types. */

import type { DocumentFilter, QueryMode } from "./query";

export interface GraphSettings {
  showLabels: boolean;
  showEdgeLabels: boolean;
  nodeSize: "small" | "medium" | "large";
  edgeThickness: "thin" | "medium" | "thick";
  layout:
    | "force"
    | "circular"
    | "random"
    | "circlepack"
    | "noverlaps"
    | "force-directed"
    | "hierarchical";
  colorBy: "type" | "community" | "degree";
  enableNodeDrag?: boolean;
  highlightNeighbors?: boolean;
  hideUnselectedEdges?: boolean;
}

export interface QuerySettings {
  mode: QueryMode;
  topK: number;
  maxTokens: number;
  temperature: number;
  stream: boolean;
  /** Enable reranking for improved retrieval precision */
  enableRerank: boolean;
  /** Top K results to keep after reranking */
  rerankTopK: number;
  /**
   * LLM provider ID to use for queries (e.g., "openai", "ollama", "lmstudio").
   * @implements SPEC-032: Provider selection in query interface
   */
  provider?: string;
  /**
   * Specific model name within the provider (e.g., "gpt-4o-mini", "gemma3:12b").
   * Combined with provider to form full model ID: "ollama/gemma3:12b"
   * @implements SPEC-032: Full model selection in query interface
   */
  model?: string;
  /**
   * Optional system prompt extension injected into the RAG prompt.
   * @implements SPEC-004: System prompt extension point
   */
  systemPrompt?: string;
  /**
   * Optional document filter to restrict RAG context to matching documents.
   * @implements SPEC-005: Document filters for queries
   */
  documentFilter?: DocumentFilter;
  /**
   * Explicitly selected document IDs for query scope restriction.
   * Rendered as dismissible pills in the query bar.
   * Merged into documentFilter.document_ids before API calls.
   * Persisted across sessions — cleared only by explicit user action.
   * @implements SPEC-031: Explicit document scope selection
   */
  scopedDocumentIds?: string[];
  /**
   * When true, stream context events request full chunk text (content_granularity: agent).
   * @implements SPEC-037
   */
  fullChunkContent?: boolean;
}

export interface IngestionSettings {
  /** Enable gleaning (multiple extraction passes) for higher quality entity extraction */
  enableGleaning: boolean;
  /** Maximum number of gleaning passes (1-3 recommended) */
  maxGleaning: number;
  /** Enable LLM-powered description summarization during merge */
  useLLMSummarization: boolean;
}

export interface AppSettings {
  theme: "light" | "dark" | "system";
  language: "en" | "zh" | "ja" | "ko";
  graphSettings: GraphSettings;
  querySettings: QuerySettings;
  ingestionSettings: IngestionSettings;
}

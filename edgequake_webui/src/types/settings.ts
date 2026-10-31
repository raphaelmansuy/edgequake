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
   * SPEC-109: reasoning effort override for this query. Undefined = Auto.
   */
  reasoningEffort?: string;
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
   * SPEC-180: kind chips of the active workspace. Derived per workspace at read time,
   * never persisted with the user's preferences.
   */
  docKinds?: string[];
  /**
   * Explicitly selected document IDs for query scope restriction.
   * Rendered as dismissible pills in the query bar.
   * Merged into documentFilter.document_ids before API calls.
   * Persisted across sessions — cleared only by explicit user action.
   * @implements SPEC-031: Explicit document scope selection
   */
  scopedDocumentIds?: string[];
  /** Display titles for `scopedDocumentIds` (chip labels; filled by `@` mentions / picker). */
  scopedDocumentTitles?: Record<string, string>;
  /**
   * When true, stream context events request full chunk text (content_granularity: agent).
   * @implements SPEC-037
   */
  fullChunkContent?: boolean;
}

/**
 * SPEC-175: browser-local preferences. Theme is owned by next-themes and
 * ingestion knobs (gleaning, summarization) were removed; see the settings store.
 */
export interface AppSettings {
  language: "en" | "zh" | "fr";
  graphSettings: GraphSettings;
  querySettings: QuerySettings;
}

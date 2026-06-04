/** Query request/response and streaming types (backend QueryMode parity). */

export type QueryMode =
  | "local"
  | "global"
  | "hybrid"
  | "naive"
  | "mix"
  | "bypass";

/** All valid query modes (backend parity). */
export const QUERY_MODES: readonly QueryMode[] = [
  "naive",
  "local",
  "global",
  "hybrid",
  "mix",
  "bypass",
] as const;

export function isQueryMode(value: string): value is QueryMode {
  return (QUERY_MODES as readonly string[]).includes(value);
}

/** Modes exposed in the query page mode selector (mix/bypass are API/advanced). */
export const QUERY_MODES_SELECTOR = [
  "local",
  "global",
  "hybrid",
  "naive",
] as const satisfies readonly QueryMode[];

export type QueryModeSelectorOption = (typeof QUERY_MODES_SELECTOR)[number];

/**
 * Document filter criteria for narrowing query scope.
 * @implements SPEC-005: Document date and pattern filters
 */
export interface DocumentFilter {
  /** Start date (inclusive) in ISO 8601 format (e.g., "2025-01-01T00:00:00Z"). */
  date_from?: string;
  /** End date (inclusive) in ISO 8601 format (e.g., "2025-12-31T23:59:59Z"). */
  date_to?: string;
  /**
   * Case-insensitive substring pattern to match against document titles.
   * Comma-separated values are treated as OR conditions.
   */
  document_pattern?: string;
}

export interface QueryRequest {
  query: string;
  mode: QueryMode;
  top_k?: number;
  max_tokens?: number;
  temperature?: number;
  stream?: boolean;
  only_context?: boolean;
  /** Optional document filter to narrow query scope. @implements SPEC-005 */
  document_filter?: DocumentFilter;
  /** Stream format version: "v1" (raw text) or "v2" (structured JSON). @implements SPEC-006 */
  stream_format?: string;
  /** LLM provider override. @implements SPEC-006 + SPEC-032 */
  llm_provider?: string;
  /** LLM model override. @implements SPEC-006 + SPEC-032 */
  llm_model?: string;
}

export interface QueryContext {
  chunks: Array<{
    content: string;
    document_id: string;
    score: number;
    file_path?: string;
    start_line?: number;
    end_line?: number;
    chunk_index?: number;
    /** Chunk UUID from storage. Used for deep-linking to document detail with selected chunk. */
    chunk_id?: string;
  }>;
  entities: Array<{
    id: string;
    label: string;
    relevance: number;
    /** Source document ID for citation link */
    source_document_id?: string;
    /** Original file path for citation display */
    source_file_path?: string;
    /** Source chunk IDs for provenance */
    source_chunk_ids?: string[];
    /** Entity type (e.g., "PERSON", "ORGANIZATION"). @implements SPEC-006 */
    entity_type?: string;
    /** Entity degree (number of relationships). @implements SPEC-006 */
    degree?: number;
  }>;
  relationships: Array<{
    source: string;
    target: string;
    type: string;
    relevance: number;
    /** Source document ID for citation link */
    source_document_id?: string;
    /** Original file path for citation display */
    source_file_path?: string;
  }>;
}

export interface QueryResponse {
  answer: string;
  context: QueryContext;
  mode: QueryMode;
  tokens_used: number;
  duration_ms: number;
}

export interface QueryStreamChunk {
  type: "token" | "context" | "thinking" | "done" | "error";
  content?: string;
  context?: QueryContext;
  error?: string;
  tokens_used?: number;
  duration_ms?: number;
  /** LLM provider used for this query (lineage tracking). @implements SPEC-032 */
  llm_provider?: string;
  /** LLM model used for this query (lineage tracking). @implements SPEC-032 */
  llm_model?: string;
  /** SPEC-006: Structured sources in context event */
  sources?: import("@/lib/api/chat").SourceReference[];
  /** SPEC-006: Query mode from context event */
  query_mode?: string;
  /** SPEC-006: Retrieval time from context event */
  retrieval_time_ms?: number;
  /** SPEC-006: Full statistics from done event */
  stats?: QueryStreamStats;
}

/** @implements SPEC-006: Query stream statistics */
export interface QueryStreamStats {
  embedding_time_ms: number;
  retrieval_time_ms: number;
  generation_time_ms: number;
  total_time_ms: number;
  sources_retrieved: number;
  tokens_used: number;
  tokens_per_second?: number;
  query_mode: string;
}

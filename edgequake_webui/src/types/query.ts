import type { SubgraphBundle } from "@/lib/utils/subgraph-types";

export type { SubgraphBundle } from "@/lib/utils/subgraph-types";

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

/** Modes exposed in the query page mode selector (full backend parity). */
export const QUERY_MODES_SELECTOR = [
  "local",
  "global",
  "hybrid",
  "mix",
  "naive",
  "bypass",
] as const satisfies readonly QueryMode[];

export type QueryModeSelectorOption = (typeof QUERY_MODES_SELECTOR)[number];

/**
 * Document filter criteria for narrowing query scope.
 * @implements SPEC-005: Document date and pattern filters
 * @implements SPEC-031: Explicit document scope selection
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
  /**
   * Explicit document IDs to restrict query scope.
   * When set, only these documents contribute RAG context (OR-unioned with
   * document_pattern; AND-combined with date filters).
   * An empty array is treated as null (no filtering).
   * @implements SPEC-031
   */
  document_ids?: string[];
  /** SPEC-180: restrict a normal workspace to these document kinds. Empty or absent means all. */
  doc_kinds?: string[];
  /** SPEC-180: restrict a normal workspace to these MIME types. */
  mime_types?: string[];
  /** SPEC-180: restrict a normal workspace to chunks from these source modalities. */
  source_modalities?: string[];
}

/** Returns true when the filter has no active criteria. */
export function isEmptyDocumentFilter(f?: DocumentFilter): boolean {
  if (!f) return true;
  return (
    !f.date_from &&
    !f.date_to &&
    !f.document_pattern &&
    (!f.document_ids || f.document_ids.length === 0) &&
    (!f.doc_kinds || f.doc_kinds.length === 0) &&
    (!f.mime_types || f.mime_types.length === 0) &&
    (!f.source_modalities || f.source_modalities.length === 0)
  );
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
  /** Optional Langfuse / GenAI session id (SPEC-124). Chat uses conversation_id instead. */
  session_id?: string;
  /** Payload tier for source snippets. @implements SPEC-037 + SPEC-028 */
  content_granularity?: 'citation' | 'agent' | 'debug';
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
    /** Prompt / API citation index `[N]` (SPEC-142). */
    reference_id?: number;
    /** PDF page number (1-indexed) where this chunk starts.
     * Present only when the source is a PDF processed with page-aware chunking.
     * Enables deep links: `/documents/{id}?page={page_start}` (SPEC-033 / SPEC-142).
     */
    page_start?: number;
    /** PDF page number where this chunk ends. May exceed page_start (SPEC-135 span). */
    page_end?: number;
    /**
     * Family of the cited document: `pdf`, `image`, `markdown`, `text`, `office`, `other`.
     * Stated by the server; do not infer it from `file_path`, which holds the title.
     */
    doc_kind?: string;
    /** MIME type of the cited document (`image/jpeg`, …). */
    mime_type?: string;
    /** What the chunk was embedded from: `text` or `image` (SPEC-180). */
    source_modality?: string;
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
    /** Human presentation for source (073). */
    source_label?: string;
    /** Human presentation for target (073). */
    target_label?: string;
    type: string;
    relevance: number;
    /** Source document ID for citation link */
    source_document_id?: string;
    /** Original file path for citation display */
    source_file_path?: string;
  }>;
  /** Optional structured subgraph retained for answer-on-graph (SPEC-155 W6). */
  subgraph?: SubgraphBundle;
}

export interface QueryResponse {
  answer: string;
  mode: QueryMode;
  sources?: import("@/lib/api/chat").SourceReference[];
  /** Structured query-matched graph (SPEC-028). */
  subgraph?: SubgraphBundle;
  /** @deprecated Legacy nested context — prefer sources + subgraph */
  context?: QueryContext;
  stats?: QueryStreamStats;
  conversation_id?: string;
  reranked?: boolean;
  /** SPEC-124: correlation trace id for Langfuse deep links */
  trace_id?: string;
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
  /** SPEC-028: Structured subgraph (entities + relationships) */
  subgraph?: SubgraphBundle;
  /** SPEC-006: Query mode from context event */
  query_mode?: string;
  /** SPEC-006: Retrieval time from context event */
  retrieval_time_ms?: number;
  /** SPEC-006: Full statistics from done event */
  stats?: QueryStreamStats;
  /** SPEC-142: verified answer markdown (document name + page links). */
  answer?: string;
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

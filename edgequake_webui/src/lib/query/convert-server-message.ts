import type { QueryContext, QueryMode, ServerMessage } from "@/types";
import type { QueryMessage } from "./query-interface-types";

function extractDocIdFromChunk(chunkId: string): string {
  const suffixIndex = chunkId.lastIndexOf("-chunk-");
  return suffixIndex > 0 ? chunkId.substring(0, suffixIndex) : chunkId;
}

/** Convert API ServerMessage to UI QueryMessage (SPEC-017 UI-P3-005). */
export function convertServerMessage(msg: ServerMessage): QueryMessage {
  let context: QueryContext | undefined;

  if (msg.context) {
    const chunkSources =
      msg.context.sources?.filter(
        (source) => source.source_type === "chunk" || !source.source_type,
      ) ?? [];

    context = {
      chunks: chunkSources.map((source) => ({
        content: source.content,
        document_id: source.document_id ?? extractDocIdFromChunk(source.id),
        score: source.score,
        file_path: source.file_path ?? source.title,
        chunk_id: source.id,
      })),
      entities:
        msg.context.entities?.map((entity) => {
          if (typeof entity === "string") {
            return { id: entity, label: entity, relevance: 1 };
          }
          return {
            id: entity.name,
            label: entity.name,
            relevance: entity.score,
            source_document_id: entity.source_document_id,
            source_file_path: entity.source_file_path,
            source_chunk_ids: entity.source_chunk_ids,
          };
        }) ?? [],
      relationships:
        msg.context.relationships?.map((relationship) => {
          if (typeof relationship === "string") {
            return {
              source: relationship,
              target: relationship,
              type: "related",
              relevance: 1,
            };
          }
          return {
            source: relationship.source,
            target: relationship.target,
            type: relationship.relation_type,
            relevance: relationship.score,
            source_document_id: relationship.source_document_id,
            source_file_path: relationship.source_file_path,
          };
        }) ?? [],
    };
  }

  return {
    id: msg.id,
    role: msg.role as "user" | "assistant",
    content: msg.content,
    mode: (msg.mode as QueryMode) ?? undefined,
    tokensUsed: msg.tokens_used ?? undefined,
    durationMs: msg.duration_ms ?? undefined,
    thinkingTimeMs: msg.thinking_time_ms ?? undefined,
    context,
    isError: msg.is_error,
    isStreaming: false,
    timestamp: new Date(msg.created_at).getTime(),
    llmProvider: msg.llm_provider ?? undefined,
    llmModel: msg.llm_model ?? undefined,
  };
}

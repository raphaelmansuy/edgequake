import type { QueryContext, QueryMode, ServerMessage } from "@/types";
import { mapServerMessageContextToQueryContext } from "@/lib/utils/source-mapper";
import type { QueryMessage } from "./query-interface-types";

/** Convert API ServerMessage to UI QueryMessage (SPEC-017 UI-P3-005). */
export function convertServerMessage(msg: ServerMessage): QueryMessage {
  let context: QueryContext | undefined;

  if (msg.context) {
    context = mapServerMessageContextToQueryContext(msg.context);
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

import type { QueryContext } from "@/types";
import type { StreamingState } from "./query-interface-types";

export interface StreamAccumulator {
  fullContent: string;
  context?: QueryContext;
  thinkingTimeMs?: number;
  newConversationId: string | null;
  streamingPhase: Extract<StreamingState, "thinking" | "generating">;
}

export interface StreamTokenUpdate {
  content: string;
  context?: QueryContext;
  thinkingTimeMs?: number;
  streamingPhase: Extract<StreamingState, "thinking" | "generating">;
}

export function createStreamAccumulator(
  conversationId: string | null,
): StreamAccumulator {
  return {
    fullContent: "",
    context: undefined,
    thinkingTimeMs: undefined,
    newConversationId: conversationId,
    streamingPhase: "thinking",
  };
}

/** Returns true when parsed response text first appears (thinking → generating). */
export function shouldTransitionToGenerating(
  hasResponseText: boolean,
  thinkingTimeMs: number | undefined,
): boolean {
  return hasResponseText && thinkingTimeMs === undefined;
}

export function applyStreamToken(
  acc: StreamAccumulator,
  tokenContent: string,
  hasResponseText: boolean,
  nowMs: number,
  thinkingStartMs: number | null,
): { accumulator: StreamAccumulator; update: StreamTokenUpdate } {
  const fullContent = acc.fullContent + tokenContent;
  let thinkingTimeMs = acc.thinkingTimeMs;
  let streamingPhase = acc.streamingPhase;

  if (
    shouldTransitionToGenerating(hasResponseText, thinkingTimeMs) &&
    thinkingStartMs != null
  ) {
    thinkingTimeMs = nowMs - thinkingStartMs;
    streamingPhase = "generating";
  }

  const accumulator: StreamAccumulator = {
    ...acc,
    fullContent,
    thinkingTimeMs,
    streamingPhase,
  };

  return {
    accumulator,
    update: {
      content: fullContent,
      context: acc.context,
      thinkingTimeMs,
      streamingPhase,
    },
  };
}

export function applyStreamContext(
  acc: StreamAccumulator,
  context: QueryContext,
): StreamAccumulator {
  return { ...acc, context };
}

export function applyStreamConversationId(
  acc: StreamAccumulator,
  conversationId: string,
): StreamAccumulator {
  return { ...acc, newConversationId: conversationId };
}

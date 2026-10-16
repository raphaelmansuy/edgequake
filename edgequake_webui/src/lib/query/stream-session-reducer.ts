/**
 * Pure stream-session state machine for query chat (SPEC-155 W7Q).
 * Applies SSE events without React coupling — easy to unit-test.
 */
import type { QueryContext, QueryMode } from "@/types";
import type { SubgraphBundle } from "@/lib/utils/subgraph-types";
import {
  applyStreamContext,
  applyStreamConversationId,
  applyStreamToken,
  createStreamAccumulator,
  type StreamAccumulator,
} from "./stream-accumulator";
import type { QueryMessage, StreamingState } from "./query-interface-types";

export type StreamStage =
  | "retrieving"
  | "reading"
  | "thinking"
  | "generating"
  | "complete"
  | "error"
  | "stopped";

export interface StreamSessionState {
  streamingState: StreamingState;
  stage: StreamStage | null;
  stageDetail?: string;
  accumulator: StreamAccumulator;
  pendingMessage: QueryMessage | null;
  optimisticUserMessage: QueryMessage | null;
  queuedMessage: string | null;
  lastSubmittedText: string | null;
  thinkingStartMs: number | null;
  /** Wall-clock when model entered a <think> block (live timer). */
  cotStartedAt: number | null;
}

export type StreamSessionEvent =
  | { type: "submit"; text: string; messageId: string; mode: QueryMode; provider?: string; model?: string }
  | { type: "conversation"; conversationId: string }
  | { type: "stage"; stage: StreamStage; detail?: string }
  | { type: "thinking"; content: string }
  | { type: "context"; context: QueryContext; subgraph?: SubgraphBundle }
  | {
      type: "token";
      content: string;
      /** True when parsed answer text exists outside any open think block */
      hasResponseText: boolean;
      /** True while an unclosed <think>/<thinking> block is open */
      cotOpen: boolean;
      nowMs: number;
    }
  | {
      type: "done";
      answer?: string;
      tokensUsed?: number;
      durationMs?: number;
      llmProvider?: string;
      llmModel?: string;
    }
  | { type: "error"; message: string }
  | { type: "abort" }
  | { type: "queue"; text: string }
  | { type: "clearQueue" }
  | { type: "reset" };

export function createStreamSession(
  conversationId: string | null = null,
): StreamSessionState {
  return {
    streamingState: "idle",
    stage: null,
    stageDetail: undefined,
    accumulator: createStreamAccumulator(conversationId),
    pendingMessage: null,
    optimisticUserMessage: null,
    queuedMessage: null,
    lastSubmittedText: null,
    thinkingStartMs: null,
    cotStartedAt: null,
  };
}

export function reduceStreamSession(
  state: StreamSessionState,
  event: StreamSessionEvent,
): StreamSessionState {
  switch (event.type) {
    case "submit": {
      const assistant: QueryMessage = {
        id: event.messageId,
        role: "assistant",
        content: "",
        mode: event.mode,
        llmProvider: event.provider,
        llmModel: event.model,
        isStreaming: true,
        timestamp: Date.now(),
      };
      return {
        ...state,
        streamingState: "thinking",
        stage: "retrieving",
        stageDetail: undefined,
        accumulator: createStreamAccumulator(state.accumulator.newConversationId),
        pendingMessage: assistant,
        optimisticUserMessage: {
          id: `optimistic-user-${Date.now()}`,
          role: "user",
          content: event.text,
          timestamp: Date.now(),
        },
        lastSubmittedText: event.text,
        thinkingStartMs: Date.now(),
        cotStartedAt: null,
        queuedMessage: null,
      };
    }

    case "conversation":
      return {
        ...state,
        accumulator: applyStreamConversationId(
          state.accumulator,
          event.conversationId,
        ),
      };

    case "stage":
      return {
        ...state,
        stage: event.stage,
        stageDetail: event.detail,
        streamingState:
          event.stage === "generating" ? "generating" : state.streamingState,
      };

    case "thinking":
      return {
        ...state,
        stage: state.stage ?? "retrieving",
        stageDetail: event.content,
      };

    case "context": {
      const next = applyStreamContext(state.accumulator, event.context);
      const pending = state.pendingMessage
        ? { ...state.pendingMessage, context: event.context }
        : null;
      return {
        ...state,
        accumulator: next,
        pendingMessage: pending,
        stage: state.stage === "retrieving" ? "reading" : state.stage,
      };
    }

    case "token": {
      if (!state.pendingMessage) return state;
      const cotStartedAt =
        event.cotOpen && state.cotStartedAt == null
          ? event.nowMs
          : event.cotOpen
            ? state.cotStartedAt
            : state.cotStartedAt;
      const { accumulator, update } = applyStreamToken(
        state.accumulator,
        event.content,
        event.hasResponseText,
        event.nowMs,
        // Prefer CoT wall-clock when available so "Thought for Ns" matches the panel
        cotStartedAt ?? state.thinkingStartMs,
      );
      const nextStage: StreamStage = event.hasResponseText
        ? "generating"
        : event.cotOpen
          ? "thinking"
          : state.stage === "thinking"
            ? "thinking"
            : "generating";
      return {
        ...state,
        accumulator,
        cotStartedAt: event.cotOpen
          ? cotStartedAt
          : event.hasResponseText
            ? state.cotStartedAt
            : cotStartedAt,
        streamingState:
          update.streamingPhase === "generating" ? "generating" : state.streamingState,
        stage: nextStage,
        pendingMessage: {
          ...state.pendingMessage,
          content: update.content,
          thinkingTimeMs: update.thinkingTimeMs,
          context: update.context ?? state.pendingMessage.context,
        },
      };
    }

    case "done": {
      if (!state.pendingMessage) {
        return {
          ...state,
          streamingState: "complete",
          stage: "complete",
          optimisticUserMessage: null,
          thinkingStartMs: null,
          cotStartedAt: null,
        };
      }
      return {
        ...state,
        streamingState: "complete",
        stage: "complete",
        pendingMessage: {
          ...state.pendingMessage,
          content: event.answer ?? state.pendingMessage.content,
          tokensUsed: event.tokensUsed,
          durationMs: event.durationMs,
          llmProvider:
            event.llmProvider ?? state.pendingMessage.llmProvider,
          llmModel: event.llmModel ?? state.pendingMessage.llmModel,
          isStreaming: false,
        },
        // The question stays until the saved thread includes it. Clearing it
        // here blanks the page when the conversation detail has not loaded.
        thinkingStartMs: null,
        cotStartedAt: null,
      };
    }

    case "error": {
      if (!state.pendingMessage) {
        return {
          ...state,
          streamingState: "error",
          stage: "error",
          optimisticUserMessage: null,
          thinkingStartMs: null,
          cotStartedAt: null,
        };
      }
      return {
        ...state,
        streamingState: "error",
        stage: "error",
        pendingMessage: {
          ...state.pendingMessage,
          content: event.message,
          isStreaming: false,
          isError: true,
        },
        optimisticUserMessage: null,
        thinkingStartMs: null,
        cotStartedAt: null,
      };
    }

    case "abort": {
      // Keep partial answer marked stopped (SPEC-155 Q08) — even if empty yet
      if (!state.pendingMessage) {
        return {
          ...state,
          streamingState: "idle",
          stage: "stopped",
          thinkingStartMs: null,
          cotStartedAt: null,
        };
      }
      return {
        ...state,
        streamingState: "idle",
        stage: "stopped",
        pendingMessage: {
          ...state.pendingMessage,
          isStreaming: false,
          stopped: true,
        },
        thinkingStartMs: null,
        cotStartedAt: null,
      };
    }

    case "queue":
      return { ...state, queuedMessage: event.text };

    case "clearQueue":
      if (state.queuedMessage == null) return state;
      return { ...state, queuedMessage: null };

    case "reset":
      return createStreamSession(state.accumulator.newConversationId);

    default:
      return state;
  }
}

/** Clear pending after server messages have been merged into conversation. */
export function clearPendingAfterMerge(
  state: StreamSessionState,
): StreamSessionState {
  return {
    ...state,
    pendingMessage: null,
    optimisticUserMessage: null,
  };
}

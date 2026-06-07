"use client";

import { parseCOTContent } from "@/components/query/thinking-display";
import { chatCompletion, chatCompletionStream } from "@/lib/api/chat";
import { deleteMessage } from "@/lib/api/conversations";
import { conversationKeys } from "@/lib/api/query-keys";
import {
  isConversationNotFoundError,
  isServerPersistedMessageId,
} from "@/lib/query/conversation-errors";
import type { QueryMessage, StreamingState } from "@/lib/query/query-interface-types";
import {
  applyStreamContext,
  applyStreamConversationId,
  applyStreamToken,
  createStreamAccumulator,
} from "@/lib/query/stream-accumulator";
import { isLlmProviderAuthFailure } from "@/lib/query-model-selection";
import { mapSourcesToContext } from "@/lib/utils/source-mapper";
import { generateUUID } from "@/lib/utils/uuid";
import type { useQueryUIStore } from "@/stores/use-query-ui-store";
import type { useSettingsStore } from "@/stores/use-settings-store";
import { useQueryClient } from "@tanstack/react-query";
import { useCallback, useRef, type Dispatch, type SetStateAction } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

type QuerySettings = ReturnType<typeof useSettingsStore.getState>["querySettings"];
type QueryUIStore = ReturnType<typeof useQueryUIStore.getState>;

interface UseQueryStreamingOptions {
  querySettings: QuerySettings;
  setQuerySettings: (settings: Partial<QuerySettings>) => void;
  activeConversationId: string | null;
  store: QueryUIStore;
  messages: QueryMessage[];
  streamingState: StreamingState;
  setStreamingState: (state: StreamingState) => void;
  setPendingMessage: Dispatch<SetStateAction<QueryMessage | null>>;
  setOptimisticUserMessage: Dispatch<SetStateAction<QueryMessage | null>>;
}

/** Streaming submit, regenerate, and abort for the query interface (SPEC-017 UI-P3-006). */
export function useQueryStreaming({
  querySettings,
  setQuerySettings,
  activeConversationId,
  store,
  messages,
  streamingState,
  setStreamingState,
  setPendingMessage,
  setOptimisticUserMessage,
}: UseQueryStreamingOptions) {
  const { t, i18n } = useTranslation();
  const queryClient = useQueryClient();
  const abortControllerRef = useRef<AbortController | null>(null);
  const thinkingStartRef = useRef<number | null>(null);

  const handleStop = useCallback(() => {
    abortControllerRef.current?.abort();
    setOptimisticUserMessage(null);
    setStreamingState("idle");
  }, [setOptimisticUserMessage, setStreamingState]);

  const handleStreamQuery = useCallback(
    async (
      queryText: string,
      conversationId: string | null,
      payloadImages?: Array<{ data: string; mime_type: string }>,
    ) => {
      const messageId = generateUUID();
      setStreamingState("thinking");
      thinkingStartRef.current = Date.now();
      abortControllerRef.current = new AbortController();

      const assistantMessage: QueryMessage = {
        id: messageId,
        role: "assistant",
        content: "",
        mode: querySettings.mode,
        isStreaming: true,
        timestamp: Date.now(),
      };
      setPendingMessage(assistantMessage);

      let accumulator = createStreamAccumulator(conversationId);

      try {
        for await (const chunk of chatCompletionStream({
          conversation_id: conversationId || undefined,
          message: queryText,
          mode: querySettings.mode,
          max_tokens: querySettings.maxTokens,
          temperature: querySettings.temperature,
          top_k: querySettings.topK,
          stream: true,
          provider: querySettings.provider,
          model: querySettings.model,
          language: i18n.language,
          system_prompt: querySettings.systemPrompt || undefined,
          document_filter: querySettings.documentFilter || undefined,
          images: payloadImages,
        })) {
          if (abortControllerRef.current?.signal.aborted) break;

          switch (chunk.type) {
            case "conversation":
              accumulator = applyStreamConversationId(
                accumulator,
                chunk.conversation_id,
              );
              if (!conversationId && chunk.conversation_id) {
                store.setActiveConversation(chunk.conversation_id);
                queryClient.invalidateQueries({
                  queryKey: conversationKeys.lists(),
                });
              }
              break;

            case "context":
              if ("sources" in chunk && chunk.sources) {
                accumulator = applyStreamContext(
                  accumulator,
                  mapSourcesToContext(chunk.sources),
                );
              }
              break;

            case "token": {
              const parsed = parseCOTContent(accumulator.fullContent + chunk.content);
              const { accumulator: next, update } = applyStreamToken(
                accumulator,
                chunk.content,
                Boolean(parsed.response),
                Date.now(),
                thinkingStartRef.current,
              );
              accumulator = next;
              if (update.streamingPhase === "generating") {
                setStreamingState("generating");
              }
              setPendingMessage({
                ...assistantMessage,
                content: update.content,
                thinkingTimeMs: update.thinkingTimeMs,
                context: update.context,
              });
              break;
            }

            case "title_update":
              queryClient.invalidateQueries({
                queryKey: conversationKeys.lists(),
              });
              if (chunk.conversation_id) {
                queryClient.invalidateQueries({
                  queryKey: conversationKeys.detail(chunk.conversation_id),
                });
              }
              break;

            case "error":
              throw new Error(chunk.message || "Streaming failed");
          }
        }

        setPendingMessage((prev) =>
          prev ? { ...prev, isStreaming: false } : null,
        );

        if (accumulator.newConversationId) {
          await queryClient.invalidateQueries({
            queryKey: conversationKeys.detail(accumulator.newConversationId),
          });
          await queryClient.invalidateQueries({
            queryKey: conversationKeys.lists(),
          });
          await new Promise((resolve) => setTimeout(resolve, 150));
        }

        setPendingMessage(null);
        setOptimisticUserMessage(null);
        setStreamingState("complete");
      } catch (error) {
        if (error instanceof Error && error.name === "AbortError") {
          setPendingMessage(null);
          setOptimisticUserMessage(null);
          setStreamingState("idle");
          return;
        }

        if (isConversationNotFoundError(error) && conversationId) {
          store.setActiveConversation(null);
          setPendingMessage(null);
          setOptimisticUserMessage(null);
          setStreamingState("idle");
          toast.warning(t("query.conversationExpired", "Conversation expired"), {
            description: t(
              "query.startingNewConversation",
              "Starting a new conversation. Please submit your query again.",
            ),
          });
          return;
        }

        const errorMessage =
          error instanceof Error ? error.message : "Query failed";

        if (isLlmProviderAuthFailure(errorMessage)) {
          setQuerySettings({ provider: undefined, model: undefined });
          toast.warning(
            t("query.modelAuthReset", "LLM provider reset"),
            {
              description: t(
                "query.modelAuthResetDesc",
                "The selected provider could not authenticate. Using the server default — submit again.",
              ),
            },
          );
        } else {
          toast.error(errorMessage, {
            action: {
              label: t("common.retry", "Retry"),
              onClick: () => {},
            },
          });
        }

        setPendingMessage({
          ...assistantMessage,
          content: errorMessage,
          isStreaming: false,
          isError: true,
        });
        setStreamingState("error");
      } finally {
        abortControllerRef.current = null;
        thinkingStartRef.current = null;
      }
    },
    [
      i18n.language,
      querySettings,
      setQuerySettings,
      queryClient,
      store,
      setPendingMessage,
      setOptimisticUserMessage,
      setStreamingState,
      t,
    ],
  );

  const submitQuery = useCallback(
    async (
      queryText: string,
      payloadImages?: Array<{ data: string; mime_type: string }>,
    ) => {
      const conversationId = activeConversationId;

      setOptimisticUserMessage({
        id: `optimistic-user-${Date.now()}`,
        role: "user",
        content: queryText,
        timestamp: Date.now(),
      });

      if (querySettings.stream) {
        await handleStreamQuery(queryText, conversationId, payloadImages);
        return;
      }

      setStreamingState("generating");
      try {
        const response = await chatCompletion({
          conversation_id: conversationId || undefined,
          message: queryText,
          mode: querySettings.mode,
          max_tokens: querySettings.maxTokens,
          temperature: querySettings.temperature,
          top_k: querySettings.topK,
          stream: false,
          provider: querySettings.provider,
          model: querySettings.model,
          language: i18n.language,
          system_prompt: querySettings.systemPrompt || undefined,
          document_filter: querySettings.documentFilter || undefined,
          images: payloadImages,
        });

        if (!conversationId && response.conversation_id) {
          store.setActiveConversation(response.conversation_id);
        }

        await queryClient.invalidateQueries({
          queryKey: conversationKeys.detail(response.conversation_id),
        });
        await queryClient.invalidateQueries({
          queryKey: conversationKeys.all,
        });
        setOptimisticUserMessage(null);
        setStreamingState("complete");
      } catch (error) {
        if (isConversationNotFoundError(error) && conversationId) {
          store.setActiveConversation(null);
          setOptimisticUserMessage(null);
          toast.warning(t("query.conversationExpired", "Conversation expired"), {
            description: t(
              "query.startingNewConversation",
              "Starting a new conversation. Please submit your query again.",
            ),
          });
          setStreamingState("idle");
          return;
        }

        setOptimisticUserMessage(null);
        const message =
          error instanceof Error
            ? error.message
            : t("common.unknownError", "Unknown error");
        if (isLlmProviderAuthFailure(message)) {
          setQuerySettings({ provider: undefined, model: undefined });
          toast.warning(t("query.modelAuthReset", "LLM provider reset"), {
            description: t(
              "query.modelAuthResetDesc",
              "The selected provider could not authenticate. Using the server default — submit again.",
            ),
          });
        } else {
          toast.error(t("query.failed", "Query failed"), {
            description: message,
          });
        }
        setStreamingState("error");
      }
    },
    [
      activeConversationId,
      handleStreamQuery,
      i18n.language,
      queryClient,
      querySettings,
      setQuerySettings,
      setOptimisticUserMessage,
      setStreamingState,
      store,
      t,
    ],
  );

  const handleRegenerate = useCallback(async () => {
    if (!activeConversationId || messages.length < 2) return;

    const lastUserMessage = [...messages].reverse().find((m) => m.role === "user");
    const lastAssistantMessage = [...messages]
      .reverse()
      .find((m) => m.role === "assistant");

    if (!lastUserMessage) return;

    const queryText = lastUserMessage.content;
    setPendingMessage(null);

    const tryDelete = async (id: string): Promise<void> => {
      if (!isServerPersistedMessageId(id)) return;
      try {
        await deleteMessage(id);
      } catch (err) {
        if (!isConversationNotFoundError(err)) {
          console.warn("Could not delete message during regeneration:", id, err);
        }
      }
    };

    try {
      const deleteOps: Promise<void>[] = [];
      if (lastAssistantMessage && !lastAssistantMessage.isStreaming) {
        deleteOps.push(tryDelete(lastAssistantMessage.id));
      }
      deleteOps.push(tryDelete(lastUserMessage.id));
      await Promise.all(deleteOps);

      await queryClient.invalidateQueries({
        queryKey: conversationKeys.detail(activeConversationId),
      });
    } catch (error) {
      console.error("Unexpected error during regeneration cleanup:", error);
    }

    await handleStreamQuery(queryText, activeConversationId);
  }, [messages, activeConversationId, handleStreamQuery, queryClient, setPendingMessage]);

  const isStreamingOrLoading =
    streamingState === "thinking" || streamingState === "generating";

  return {
    handleStop,
    handleRegenerate,
    submitQuery,
    isStreamingOrLoading,
  };
}

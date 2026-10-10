"use client";

/**
 * Stream session orchestration for the query interface (SPEC-155 W7Q).
 * Thin React wrapper over stream-session-reducer + buildChatRequest.
 */
import { useLlmModels } from "@/hooks/use-providers";
import { chatCompletion, chatCompletionStream } from "@/lib/api/chat";
import {
  modelSupportsStreaming,
  resolveStreamMode,
} from "@/lib/query/model-menu";
import { parseCOTStreaming } from "@/lib/query/parse-cot-streaming";
import { deleteMessage } from "@/lib/api/conversations";
import { conversationKeys } from "@/lib/api/query-keys";
import { buildChatRequest } from "@/lib/query/build-chat-request";
import { seedEntityIdsFromCompanionTarget } from "@/lib/query/companion-pane";
import {
  errorMessageOf,
  handleConversationRecovery,
  handleProviderAuthRecovery,
  type RecoveryActions,
  type RecoveryCopy,
} from "@/lib/query/conversation-recovery";
import {
  isConversationNotFoundError,
  isServerPersistedMessageId,
} from "@/lib/query/conversation-errors";
import type { QueryMessage } from "@/lib/query/query-interface-types";
import { conversationEchoesPending } from "@/lib/query/merge-query-messages";
import {
  clearPendingAfterMerge,
  createStreamSession,
  reduceStreamSession,
  type StreamSessionState,
  type StreamStage,
} from "@/lib/query/stream-session-reducer";
import { buildQueryContextFromRetrieval } from "@/lib/utils/source-mapper";
import { generateUUID } from "@/lib/utils/uuid";
import { useAnswerGraphStore } from "@/stores/use-answer-graph-store";
import { useCompanionPaneStore } from "@/stores/use-companion-pane-store";
import type { useQueryUIStore } from "@/stores/use-query-ui-store";
import type { useSettingsStore } from "@/stores/use-settings-store";
import { useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

/** Companion Ask entity id for seed_entity_ids (mode unchanged). */
function companionSeedEntityIds(): string[] | undefined {
  return seedEntityIdsFromCompanionTarget(useCompanionPaneStore.getState().target);
}

type QuerySettings = ReturnType<typeof useSettingsStore.getState>["querySettings"];
type QueryUIStore = ReturnType<typeof useQueryUIStore.getState>;

interface Options {
  querySettings: QuerySettings;
  setQuerySettings: (settings: Partial<QuerySettings>) => void;
  activeConversationId: string | null;
  store: QueryUIStore;
  messages: QueryMessage[];
}

export function useQueryStreamSession({
  querySettings,
  setQuerySettings,
  activeConversationId,
  store,
  messages,
}: Options) {
  const { t, i18n } = useTranslation();
  const queryClient = useQueryClient();
  const abortRef = useRef<AbortController | null>(null);
  const editFromMessageIdRef = useRef<string | null>(null);
  /** Late-bound `runQuery` so toast Retry actions can re-dispatch without self-reference. */
  const rerunRef = useRef<
    (
      text: string,
      conversationId: string | null,
      images?: Array<{ data: string; mime_type: string }>,
    ) => Promise<void>
  >(async () => {});
  const [session, setSession] = useState<StreamSessionState>(() =>
    createStreamSession(activeConversationId),
  );
  /** Bumped on abort/Ask so a late SSE `conversation` cannot re-attach (EC-159-06). */
  const requestGenRef = useRef(0);

  useEffect(() => {
    return () => {
      abortRef.current?.abort();
      requestGenRef.current += 1;
    };
  }, []);

  // Stream when the user wants it and the selected (or default) model supports it.
  const { data: llmCatalog } = useLlmModels();
  const effectiveStream = resolveStreamMode(
    querySettings.stream,
    modelSupportsStreaming(llmCatalog?.models, {
      provider: querySettings.provider,
      model: querySettings.model,
      defaultProvider: llmCatalog?.default_provider,
      defaultModel: llmCatalog?.default_model,
    }),
  );

  const tryDeleteMessage = useCallback(async (id: string) => {
    if (!isServerPersistedMessageId(id)) return;
    try {
      await deleteMessage(id);
    } catch (err) {
      if (!isConversationNotFoundError(err)) {
        console.warn("Could not delete message:", id, err);
      }
    }
  }, []);

  const copy: RecoveryCopy = {
    conversationExpired: t("query.conversationExpired", "Conversation expired"),
    startingNewConversation: t(
      "query.startingNewConversation",
      "Starting a new conversation. Please submit your query again.",
    ),
    modelAuthReset: t("query.modelAuthReset", "LLM provider reset"),
    modelAuthResetDesc: t(
      "query.modelAuthResetDesc",
      "The selected provider could not authenticate. Using the server default — submit again.",
    ),
  };

  const recoveryActions = useCallback((): RecoveryActions => {
    return {
      clearActiveConversation: () => store.setActiveConversation(null),
      clearPending: () =>
        setSession((s) => ({
          ...clearPendingAfterMerge(s),
          streamingState: "idle",
          stage: null,
        })),
      resetProviderModel: () =>
        setQuerySettings({ provider: undefined, model: undefined }),
      toastWarning: (title, description) =>
        toast.warning(title, { description }),
    };
  }, [setQuerySettings, store]);

  const patch = useCallback((updater: (s: StreamSessionState) => StreamSessionState) => {
    setSession(updater);
  }, []);

  const handleStop = useCallback(() => {
    abortRef.current?.abort();
    requestGenRef.current += 1;
    setSession((s) => reduceStreamSession(s, { type: "abort" }));
  }, []);

  /** SPEC-159: drop in-flight answer chrome when Ask starts a new conversation. */
  const abandonInFlight = useCallback(() => {
    abortRef.current?.abort();
    requestGenRef.current += 1;
    editFromMessageIdRef.current = null;
    setSession((s) => ({
      ...reduceStreamSession(s, { type: "abort" }),
      pendingMessage: null,
      optimisticUserMessage: null,
      queuedMessage: null,
    }));
  }, []);

  const handleStreamQuery = useCallback(
    async (
      queryText: string,
      conversationId: string | null,
      payloadImages?: Array<{ data: string; mime_type: string }>,
    ) => {
      const messageId = generateUUID();
      const gen = requestGenRef.current;
      abortRef.current = new AbortController();

      setSession((s) =>
        reduceStreamSession(s, {
          type: "submit",
          text: queryText,
          messageId,
          mode: querySettings.mode,
          provider: querySettings.provider,
          model: querySettings.model,
        }),
      );

      try {
        const request = buildChatRequest({
          settings: querySettings,
          message: queryText,
          conversationId,
          language: i18n.language,
          images: payloadImages,
          stream: true,
          seedEntityIds: companionSeedEntityIds(),
        });

        for await (const chunk of chatCompletionStream(request)) {
          if (abortRef.current?.signal.aborted || gen !== requestGenRef.current) {
            break;
          }

          switch (chunk.type) {
            case "conversation":
              setSession((s) =>
                reduceStreamSession(s, {
                  type: "conversation",
                  conversationId: chunk.conversation_id,
                }),
              );
              if (
                !conversationId &&
                chunk.conversation_id &&
                gen === requestGenRef.current
              ) {
                store.setActiveConversation(chunk.conversation_id);
                queryClient.invalidateQueries({
                  queryKey: conversationKeys.lists(),
                });
              }
              break;

            case "stage":
              setSession((s) =>
                reduceStreamSession(s, {
                  type: "stage",
                  stage: chunk.stage as StreamStage,
                  detail: chunk.detail,
                }),
              );
              break;

            case "thinking":
              setSession((s) =>
                reduceStreamSession(s, {
                  type: "thinking",
                  content: chunk.content,
                }),
              );
              break;

            case "context": {
              const hasSources = (chunk.sources?.length ?? 0) > 0;
              const hasSubgraph =
                (chunk.subgraph?.entities?.length ?? 0) > 0 ||
                (chunk.subgraph?.relationships?.length ?? 0) > 0;
              const context = buildQueryContextFromRetrieval(
                chunk.sources ?? [],
                chunk.subgraph,
              );
              // Always apply context (incl. empty) so UI can show "no sources"
              if (hasSources || hasSubgraph || chunk.sources) {
                setSession((s) =>
                  reduceStreamSession(s, { type: "context", context }),
                );
              }
              if (hasSubgraph && chunk.subgraph) {
                useAnswerGraphStore
                  .getState()
                  .recordSubgraph(messageId, chunk.subgraph);
              }
              break;
            }

            case "token": {
              setSession((s) => {
                const nextContent =
                  (s.accumulator.fullContent || "") + chunk.content;
                const parsed = parseCOTStreaming(nextContent);
                const hasResponse = Boolean(parsed.response.trim());
                return reduceStreamSession(s, {
                  type: "token",
                  content: chunk.content,
                  hasResponseText: hasResponse,
                  cotOpen: parsed.open,
                  nowMs: Date.now(),
                });
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

            case "done":
              setSession((s) =>
                reduceStreamSession(s, {
                  type: "done",
                  answer: chunk.answer,
                  tokensUsed: chunk.tokens_used,
                  durationMs: chunk.duration_ms,
                  llmProvider: chunk.llm_provider ?? querySettings.provider,
                  llmModel: chunk.llm_model ?? querySettings.model,
                }),
              );
              break;

            case "error":
              if (
                handleConversationRecovery(
                  new Error(chunk.message),
                  recoveryActions(),
                  copy,
                  chunk.code,
                )
              ) {
                return;
              }
              setSession((s) =>
                reduceStreamSession(s, {
                  type: "error",
                  message: chunk.message || "Streaming failed",
                }),
              );
              return;
          }
        }

        if (abortRef.current?.signal.aborted || gen !== requestGenRef.current) {
          return;
        }

        let settledId = conversationId;
        setSession((s) => {
          settledId = s.accumulator.newConversationId || settledId;
          return s;
        });
        if (settledId) {
          await queryClient.invalidateQueries({
            queryKey: conversationKeys.detail(settledId),
          });
          await queryClient.invalidateQueries({
            queryKey: conversationKeys.lists(),
          });
        }
        setSession((s) => {
          if (s.stage === "stopped") return s;
          const pendingContent = s.pendingMessage?.content;
          if (!settledId || !pendingContent) return clearPendingAfterMerge(s);
          const cached = queryClient.getQueryData<{
            messages?: { role?: string; content?: string }[];
          }>(conversationKeys.detail(settledId));
          return conversationEchoesPending(cached?.messages, pendingContent)
            ? clearPendingAfterMerge(s)
            : s;
        });
      } catch (error) {
        if (error instanceof Error && error.name === "AbortError") {
          setSession((s) => reduceStreamSession(s, { type: "abort" }));
          return;
        }

        if (
          handleConversationRecovery(
            error,
            recoveryActions(),
            copy,
            undefined,
          ) &&
          conversationId
        ) {
          return;
        }

        const msg = errorMessageOf(error);
        if (handleProviderAuthRecovery(msg, recoveryActions(), copy)) {
          setSession((s) =>
            reduceStreamSession(s, { type: "error", message: msg }),
          );
          return;
        }

        toast.error(msg, {
          action: {
            label: t("common.retry", "Retry"),
            onClick: () => {
              void rerunRef.current(queryText, conversationId, payloadImages);
            },
          },
        });

        setSession((s) =>
          reduceStreamSession(s, { type: "error", message: msg }),
        );
      } finally {
        abortRef.current = null;
      }
    },
    [
      activeConversationId,
      copy,
      i18n.language,
      queryClient,
      querySettings,
      recoveryActions,
      store,
      t,
    ],
  );

  const truncateFromEditAnchor = useCallback(async () => {
    const editId = editFromMessageIdRef.current;
    if (!editId || !activeConversationId) return;
    const startIdx = messages.findIndex((m) => m.id === editId);
    if (startIdx < 0) {
      editFromMessageIdRef.current = null;
      return;
    }
    for (const m of messages.slice(startIdx)) {
      await tryDeleteMessage(m.id);
    }
    editFromMessageIdRef.current = null;
    await queryClient.invalidateQueries({
      queryKey: conversationKeys.detail(activeConversationId),
    });
  }, [activeConversationId, messages, queryClient, tryDeleteMessage]);

  const beginEditFromMessage = useCallback(
    (messageId: string) => {
      const msg = messages.find((m) => m.id === messageId);
      if (!msg || msg.role !== "user") return "";
      editFromMessageIdRef.current = messageId;
      return msg.content;
    },
    [messages],
  );

  const clearEditAnchor = useCallback(() => {
    editFromMessageIdRef.current = null;
  }, []);

  /** Non-streaming path: one request, answer arrives at once. */
  const handleBlockingQuery = useCallback(
    async (
      queryText: string,
      conversationId: string | null,
      payloadImages?: Array<{ data: string; mime_type: string }>,
    ) => {
      const gen = requestGenRef.current;
      setSession((s) =>
        reduceStreamSession(s, {
          type: "submit",
          text: queryText,
          messageId: generateUUID(),
          mode: querySettings.mode,
          provider: querySettings.provider,
          model: querySettings.model,
        }),
      );
      setSession((s) =>
        reduceStreamSession(s, {
          type: "stage",
          stage: "generating",
        }),
      );

      try {
        const response = await chatCompletion(
          buildChatRequest({
            settings: querySettings,
            message: queryText,
            conversationId,
            language: i18n.language,
            images: payloadImages,
            stream: false,
            seedEntityIds: companionSeedEntityIds(),
          }),
        );

        if (gen !== requestGenRef.current) {
          return;
        }

        if (!conversationId && response.conversation_id) {
          store.setActiveConversation(response.conversation_id);
        }

        await queryClient.invalidateQueries({
          queryKey: conversationKeys.detail(response.conversation_id),
        });
        await queryClient.invalidateQueries({
          queryKey: conversationKeys.all,
        });
        setSession((s) =>
          clearPendingAfterMerge(
            reduceStreamSession(s, {
              type: "done",
              tokensUsed: response.tokens_used,
              durationMs: response.duration_ms,
              llmProvider: response.llm_provider,
              llmModel: response.llm_model,
              answer: response.content,
            }),
          ),
        );
      } catch (error) {
        if (
          conversationId &&
          handleConversationRecovery(error, recoveryActions(), copy)
        ) {
          return;
        }
        const message = errorMessageOf(
          error,
          t("common.unknownError", "Unknown error"),
        );
        if (!handleProviderAuthRecovery(message, recoveryActions(), copy)) {
          toast.error(t("query.failed", "Query failed"), {
            description: message,
            action: {
              label: t("common.retry", "Retry"),
              onClick: () => {
                void rerunRef.current(queryText, conversationId, payloadImages);
              },
            },
          });
        }
        setSession((s) =>
          reduceStreamSession(s, { type: "error", message }),
        );
      }
    },
    [
      copy,
      i18n.language,
      queryClient,
      querySettings,
      recoveryActions,
      store,
      t,
    ],
  );

  /** Single dispatch point: stream when the user wants it AND the model can. */
  const runQuery = useCallback(
    async (
      queryText: string,
      conversationId: string | null,
      payloadImages?: Array<{ data: string; mime_type: string }>,
    ) => {
      if (effectiveStream) {
        await handleStreamQuery(queryText, conversationId, payloadImages);
      } else {
        await handleBlockingQuery(queryText, conversationId, payloadImages);
      }
    },
    [effectiveStream, handleBlockingQuery, handleStreamQuery],
  );

  useEffect(() => {
    rerunRef.current = runQuery;
  }, [runQuery]);

  const submitQuery = useCallback(
    async (
      queryText: string,
      payloadImages?: Array<{ data: string; mime_type: string }>,
    ) => {
      await truncateFromEditAnchor();
      await runQuery(queryText, activeConversationId, payloadImages);
    },
    [activeConversationId, runQuery, truncateFromEditAnchor],
  );

  const handleRegenerate = useCallback(async () => {
    if (!activeConversationId || messages.length < 2) return;

    const lastUserMessage = [...messages].reverse().find((m) => m.role === "user");
    const lastAssistantMessage = [...messages]
      .reverse()
      .find((m) => m.role === "assistant");

    if (!lastUserMessage) return;

    const queryText = lastUserMessage.content;
    // Non-destructive: only delete assistant message (keep user). Soft versions later.
    if (lastAssistantMessage && !lastAssistantMessage.isStreaming) {
      await tryDeleteMessage(lastAssistantMessage.id);
    }
    // Keep user message — do NOT delete (Q12 fix)
    await queryClient.invalidateQueries({
      queryKey: conversationKeys.detail(activeConversationId),
    });

    await runQuery(queryText, activeConversationId);
  }, [messages, activeConversationId, runQuery, queryClient, tryDeleteMessage]);

  const handleRetry = useCallback(() => {
    const text = session.lastSubmittedText;
    if (!text) return;
    void submitQuery(text);
  }, [session.lastSubmittedText, submitQuery]);

  const queueMessage = useCallback((text: string) => {
    setSession((s) => reduceStreamSession(s, { type: "queue", text }));
  }, []);

  const clearQueue = useCallback(() => {
    setSession((s) => reduceStreamSession(s, { type: "clearQueue" }));
  }, []);

  const isStreamingOrLoading =
    session.streamingState === "thinking" ||
    session.streamingState === "generating";

  return {
    streamingState: session.streamingState,
    stage: session.stage,
    stageDetail: session.stageDetail,
    pendingMessage: session.pendingMessage,
    optimisticUserMessage: session.optimisticUserMessage,
    queuedMessage: session.queuedMessage,
    lastSubmittedText: session.lastSubmittedText,
    setPendingMessage: (msg: QueryMessage | null) =>
      setSession((s) => ({ ...s, pendingMessage: msg })),
    setOptimisticUserMessage: (msg: QueryMessage | null) =>
      setSession((s) => ({ ...s, optimisticUserMessage: msg })),
    handleStop,
    abandonInFlight,
    handleRegenerate,
    handleRetry,
    beginEditFromMessage,
    clearEditAnchor,
    submitQuery,
    queueMessage,
    clearQueue,
    isStreamingOrLoading,
    patch,
  };
}

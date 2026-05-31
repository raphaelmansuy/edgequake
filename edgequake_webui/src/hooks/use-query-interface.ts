"use client";

import { useConversation, useConversations } from "@/hooks/use-conversations";
import { useQueryImages } from "@/hooks/use-query-images";
import { useProvidersHealth } from "@/hooks/use-models";
import { useLlmModels as useProviderLlmModels } from "@/hooks/use-providers";
import { chatCompletion, chatCompletionStream } from "@/lib/api/chat";
import { deleteMessage } from "@/lib/api/conversations";
import { conversationKeys } from "@/lib/api/query-keys";
import {
  isConversationNotFoundError,
  isServerPersistedMessageId,
} from "@/lib/query/conversation-errors";
import { convertServerMessage } from "@/lib/query/convert-server-message";
import { mergeQueryMessages } from "@/lib/query/merge-query-messages";
import type {
  QueryMessage,
  StreamingState,
} from "@/lib/query/query-interface-types";
import { sanitizeQueryModelSelection } from "@/lib/query-model-selection";
import { mapSourcesToContext } from "@/lib/utils/source-mapper";
import { generateUUID } from "@/lib/utils/uuid";
import { useActiveConversationId, useQueryUIStore } from "@/stores/use-query-ui-store";
import { useSettingsStore } from "@/stores/use-settings-store";
import { useTenantStore } from "@/stores/use-tenant-store";
import { useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { parseCOTContent } from "@/components/query/thinking-display";

export function useQueryInterface() {
  const { t, i18n } = useTranslation();
  const [input, setInput] = useState("");
  const [streamingState, setStreamingState] = useState<StreamingState>("idle");
  const [shouldAutoScroll, setShouldAutoScroll] = useState(true);
  const [pendingMessage, setPendingMessage] = useState<QueryMessage | null>(null);
  const [optimisticUserMessage, setOptimisticUserMessage] =
    useState<QueryMessage | null>(null);

  const scrollRef = useRef<HTMLDivElement>(null);
  const scrollAnchorRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const abortControllerRef = useRef<AbortController | null>(null);
  const thinkingStartRef = useRef<number | null>(null);
  const hasInitializedRef = useRef(false);
  const hasResetUnavailableModelRef = useRef(false);

  const queryClient = useQueryClient();
  const { querySettings, setQuerySettings } = useSettingsStore();
  const { selectedTenantId, selectedWorkspaceId } = useTenantStore();
  const { data: llmCatalog } = useProviderLlmModels();
  const { data: providerHealth } = useProvidersHealth({
    enabled: true,
    refetchInterval: 60 * 1000,
  });

  const store = useQueryUIStore();
  const activeConversationId = useActiveConversationId();

  const {
    data: activeConversation,
    isLoading: isLoadingConversation,
    error: conversationError,
    isError: isConversationError,
  } = useConversation(activeConversationId);

  const { data: conversationsData } = useConversations({
    sort: "updated_at",
  });

  const images = useQueryImages();

  useEffect(() => {
    if (!isConversationError || !activeConversationId) return;
    if (!isConversationNotFoundError(conversationError)) return;

    store.setActiveConversation(null);
    toast(t("query.conversationExpired", "Previous conversation not available"), {
      description: t(
        "query.startingFreshSession",
        "Starting a fresh session.",
      ),
    });
  }, [isConversationError, conversationError, activeConversationId, store, t]);

  useEffect(() => {
    const sanitizedSelection = sanitizeQueryModelSelection(
      {
        provider: querySettings.provider,
        model: querySettings.model,
      },
      llmCatalog?.models,
      providerHealth,
    );

    if (
      sanitizedSelection.provider !== querySettings.provider ||
      sanitizedSelection.model !== querySettings.model
    ) {
      setQuerySettings(sanitizedSelection);

      if (
        !hasResetUnavailableModelRef.current &&
        (querySettings.provider || querySettings.model)
      ) {
        hasResetUnavailableModelRef.current = true;
        toast.warning(t("query.modelReset", "Model selection reset"), {
          description: t(
            "query.modelResetDesc",
            "Your previous model is unavailable in this environment, so the server default will be used.",
          ),
        });
      }
    }
  }, [
    llmCatalog?.models,
    providerHealth,
    querySettings.model,
    querySettings.provider,
    setQuerySettings,
    t,
  ]);

  useEffect(() => {
    if (hasInitializedRef.current) return;
    hasInitializedRef.current = true;

    const firstPage = conversationsData?.pages?.[0];
    if (!activeConversationId && firstPage?.items && firstPage.items.length > 0) {
      store.setActiveConversation(firstPage.items[0].id);
    }
  }, [activeConversationId, conversationsData, store]);

  const messages = useMemo(() => {
    const serverMessages = (activeConversation?.messages ?? []).map(
      convertServerMessage,
    );
    return mergeQueryMessages(
      serverMessages,
      optimisticUserMessage,
      pendingMessage,
    );
  }, [
    activeConversation?.messages,
    pendingMessage,
    optimisticUserMessage,
  ]);

  useEffect(() => {
    if (activeConversationId && messages.length > 0) {
      store.setActiveConversation(null);
      setPendingMessage(null);
      toast(t("query.conversationCleared", "New conversation started"), {
        description: t(
          "query.conversationClearedDesc",
          "Context has changed. Starting a fresh conversation.",
        ),
      });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedTenantId, selectedWorkspaceId]);

  useEffect(() => {
    if (!shouldAutoScroll) return;
    scrollAnchorRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
  }, [messages, streamingState, shouldAutoScroll]);

  useEffect(() => {
    const viewport = scrollRef.current?.querySelector(
      "[data-radix-scroll-area-viewport]",
    );
    if (!viewport) return;

    const handleScroll = () => {
      const { scrollTop, scrollHeight, clientHeight } = viewport as HTMLElement;
      const isNearBottom = scrollHeight - scrollTop - clientHeight < 100;
      setShouldAutoScroll(isNearBottom);
    };

    viewport.addEventListener("scroll", handleScroll);
    return () => viewport.removeEventListener("scroll", handleScroll);
  }, []);

  useEffect(() => {
    if (streamingState === "thinking" || streamingState === "generating") {
      setShouldAutoScroll(true);
    }
  }, [streamingState]);

  const handleInputChange = useCallback(
    (event: React.ChangeEvent<HTMLTextAreaElement>) => {
      setInput(event.target.value);
      event.target.style.height = "auto";
      event.target.style.height = `${Math.min(event.target.scrollHeight, 200)}px`;
    },
    [],
  );

  const handleStop = useCallback(() => {
    abortControllerRef.current?.abort();
    setOptimisticUserMessage(null);
    setStreamingState("idle");
  }, []);

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

      try {
        let fullContent = "";
        let context: QueryMessage["context"];
        let thinkingTimeMs: number | undefined;
        let newConversationId = conversationId;

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
              newConversationId = chunk.conversation_id;
              if (!conversationId && newConversationId) {
                store.setActiveConversation(newConversationId);
                queryClient.invalidateQueries({
                  queryKey: conversationKeys.lists(),
                });
              }
              break;

            case "context":
              if ("sources" in chunk && chunk.sources) {
                context = mapSourcesToContext(chunk.sources);
              }
              break;

            case "token":
              fullContent += chunk.content;
              {
                const parsed = parseCOTContent(fullContent);
                if (
                  parsed.response &&
                  !thinkingTimeMs &&
                  thinkingStartRef.current
                ) {
                  thinkingTimeMs = Date.now() - thinkingStartRef.current;
                  setStreamingState("generating");
                }
              }
              setPendingMessage({
                ...assistantMessage,
                content: fullContent,
                thinkingTimeMs,
                context,
              });
              break;

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

        if (newConversationId) {
          await queryClient.invalidateQueries({
            queryKey: conversationKeys.detail(newConversationId),
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
        toast.error(errorMessage, {
          action: {
            label: t("common.retry", "Retry"),
            onClick: () => {},
          },
        });

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
    [i18n.language, querySettings, queryClient, store, t],
  );

  const handleSubmit = async (event?: React.FormEvent) => {
    event?.preventDefault();

    const isStreamingOrLoading =
      streamingState === "thinking" || streamingState === "generating";
    if (!input.trim() || isStreamingOrLoading) return;

    const queryText = input.trim();
    const currentImages = images.getPayload();
    setInput("");
    images.clearImages();

    if (inputRef.current) {
      inputRef.current.style.height = "auto";
    }

    setOptimisticUserMessage({
      id: `optimistic-user-${Date.now()}`,
      role: "user",
      content: queryText,
      timestamp: Date.now(),
    });

    const conversationId = activeConversationId;

    if (querySettings.stream) {
      await handleStreamQuery(queryText, conversationId, currentImages);
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
        images: currentImages,
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
      toast.error(t("query.failed", "Query failed"), {
        description:
          error instanceof Error
            ? error.message
            : t("common.unknownError", "Unknown error"),
      });
      setStreamingState("error");
    }
  };

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
  }, [messages, activeConversationId, handleStreamQuery, queryClient]);

  const handleSuggestionClick = useCallback((text: string) => {
    setInput(text);
    inputRef.current?.focus();
  }, []);

  const handleNewConversation = useCallback(() => {
    store.setActiveConversation(null);
    setPendingMessage(null);
    setOptimisticUserMessage(null);
    setInput("");
    setStreamingState("idle");
  }, [store]);

  const isLoading =
    streamingState === "thinking" ||
    streamingState === "generating" ||
    isLoadingConversation;

  return {
    input,
    streamingState,
    pendingMessage,
    messages,
    isLoading,
    querySettings,
    setQuerySettings,
    scrollRef,
    scrollAnchorRef,
    inputRef,
    handleInputChange,
    handleSubmit,
    handleStop,
    handleRegenerate,
    handleSuggestionClick,
    handleNewConversation,
    ...images,
  };
}

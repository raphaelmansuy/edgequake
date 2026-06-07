"use client";

import { useQueryConversationLifecycle } from "@/hooks/use-query-conversation-lifecycle";
import { useQueryImages } from "@/hooks/use-query-images";
import { useQueryScroll } from "@/hooks/use-query-scroll";
import { useQueryStreaming } from "@/hooks/use-query-streaming";
import { convertServerMessage } from "@/lib/query/convert-server-message";
import { mergeQueryMessages } from "@/lib/query/merge-query-messages";
import type {
  QueryMessage,
  StreamingState,
} from "@/lib/query/query-interface-types";
import { useCallback, useMemo, useRef, useState } from "react";

/** Composes query page lifecycle, streaming, scroll, and image hooks (SPEC-017 UI-P3-006). */
export function useQueryInterface() {
  const [input, setInput] = useState("");
  const [streamingState, setStreamingState] = useState<StreamingState>("idle");
  const [pendingMessage, setPendingMessage] = useState<QueryMessage | null>(null);
  const [optimisticUserMessage, setOptimisticUserMessage] =
    useState<QueryMessage | null>(null);

  const inputRef = useRef<HTMLTextAreaElement>(null);

  const clearPendingState = useCallback(() => {
    setPendingMessage(null);
  }, []);

  const {
    activeConversation,
    activeConversationId,
    isLoadingConversation,
    querySettings,
    setQuerySettings,
    store,
  } = useQueryConversationLifecycle({
    onTenantContextChange: clearPendingState,
  });

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

  const { scrollRef, scrollAnchorRef } = useQueryScroll(streamingState, messages);

  const images = useQueryImages();

  const { handleStop, handleRegenerate, submitQuery, isStreamingOrLoading } =
    useQueryStreaming({
      querySettings,
      setQuerySettings,
      activeConversationId,
      store,
      messages,
      streamingState,
      setStreamingState,
      setPendingMessage,
      setOptimisticUserMessage,
    });

  const handleInputChange = useCallback(
    (event: React.ChangeEvent<HTMLTextAreaElement>) => {
      setInput(event.target.value);
      event.target.style.height = "auto";
      event.target.style.height = `${Math.min(event.target.scrollHeight, 200)}px`;
    },
    [],
  );

  const handleSubmit = async (event?: React.FormEvent) => {
    event?.preventDefault();
    if (!input.trim() || isStreamingOrLoading) return;

    const queryText = input.trim();
    const currentImages = images.getPayload();
    setInput("");
    images.clearImages();

    if (inputRef.current) {
      inputRef.current.style.height = "auto";
    }

    await submitQuery(queryText, currentImages);
  };

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

  const isLoading = isStreamingOrLoading || isLoadingConversation;

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

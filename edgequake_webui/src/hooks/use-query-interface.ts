"use client";

import { useQueryConversationLifecycle } from "@/hooks/use-query-conversation-lifecycle";
import { useQueryImages } from "@/hooks/use-query-images";
import { searchImageMessages, useQuerySearchImage } from "@/hooks/use-query-search-image";
import { useQueryStreamSession } from "@/hooks/use-query-stream-session";
import { useStickToBottom } from "@/hooks/use-stick-to-bottom";
import { convertServerMessage } from "@/lib/query/convert-server-message";
import { mergeQueryMessages } from "@/lib/query/merge-query-messages";
import {
  beginNewQueryConversation,
  endQueryHandoffNewConversation,
  QUERY_HANDOFF_APPLIED_EVENT,
  QUERY_HANDOFF_DRAFT_KEY,
  QUERY_HANDOFF_FOCUS_KEY,
} from "@/lib/query/query-handoff";
import type { QueryMessage } from "@/lib/query/query-interface-types";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

/** Composes query page lifecycle, streaming, scroll, and image hooks (SPEC-155 W7Q). */
export function useQueryInterface() {
  const { t } = useTranslation();
  const [input, setInput] = useState("");
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const draftKeyRef = useRef<string>("query-draft:new");
  /** Skip one persist after hydrate so Strict Mode remount cannot wipe Ask seeds. */
  const skipDraftPersistRef = useRef(false);

  const clearPendingState = useCallback(() => {
    // no-op placeholder — stream session owns pending
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

  // Per-conversation draft persistence (Q18) + SPEC-159 Ask seed consume.
  useEffect(() => {
    draftKeyRef.current = `query-draft:${activeConversationId ?? "new"}`;
    skipDraftPersistRef.current = true;
    try {
      const saved = localStorage.getItem(draftKeyRef.current);
      if (saved != null) setInput(saved);
      else setInput("");
    } catch {
      /* ignore */
    }
  }, [activeConversationId]);

  // SPEC-159: one-shot focus after cross-route Ask (never auto-submit).
  useEffect(() => {
    try {
      if (sessionStorage.getItem(QUERY_HANDOFF_FOCUS_KEY) !== "1") return;
      sessionStorage.removeItem(QUERY_HANDOFF_FOCUS_KEY);
    } catch {
      return;
    }
    requestAnimationFrame(() => {
      const el = inputRef.current;
      if (!el) return;
      el.focus();
      const len = el.value.length;
      el.setSelectionRange(len, len);
    });
  }, [activeConversationId, input]);

  useEffect(() => {
    if (skipDraftPersistRef.current) {
      skipDraftPersistRef.current = false;
      return;
    }
    try {
      if (input) localStorage.setItem(draftKeyRef.current, input);
      else localStorage.removeItem(draftKeyRef.current);
    } catch {
      /* ignore */
    }
  }, [input]);

  const baseMessages = useMemo(() => {
    return (activeConversation?.messages ?? []).map(convertServerMessage);
  }, [activeConversation?.messages]);

  // Temporary messages list for regenerate (before stream session merges)
  //
  // WHY the render-phase compare: this held a copy of `baseMessages` in state, re-synced by an
  // effect, so every conversation switch rendered once with the *previous* conversation's messages
  // and then again with the right ones — a visible flash in the transcript. Adjusting during
  // render keeps the two in step on the first pass, which is what React's "adjust state when a
  // prop changes" pattern exists for.
  const [messagesForHook, setMessagesForHook] = useState<QueryMessage[]>(baseMessages);
  const [messagesSource, setMessagesSource] = useState(baseMessages);
  if (messagesSource !== baseMessages) {
    setMessagesSource(baseMessages);
    setMessagesForHook(baseMessages);
  }

  const stream = useQueryStreamSession({
    querySettings,
    setQuerySettings,
    activeConversationId,
    store,
    messages: messagesForHook,
  });

  // SPEC-159: Ask always starts a new conversation. When already on /query with
  // activeConversationId === null, the draft effect does not re-run — this event
  // reapplies seed + focus and abandons in-flight stream (EC-159-06).
  const abandonInFlight = stream.abandonInFlight;
  useEffect(() => {
    const applyHandoff = () => {
      skipDraftPersistRef.current = true;
      store.setActiveConversation(null);
      abandonInFlight();
      try {
        const saved = localStorage.getItem(QUERY_HANDOFF_DRAFT_KEY);
        if (saved != null) setInput(saved);
      } catch {
        /* ignore */
      }
      try {
        if (sessionStorage.getItem(QUERY_HANDOFF_FOCUS_KEY) === "1") {
          sessionStorage.removeItem(QUERY_HANDOFF_FOCUS_KEY);
        }
      } catch {
        /* ignore */
      }
      requestAnimationFrame(() => {
        const el = inputRef.current;
        if (!el) return;
        el.focus();
        const len = el.value.length;
        el.setSelectionRange(len, len);
      });
    };
    window.addEventListener(QUERY_HANDOFF_APPLIED_EVENT, applyHandoff);
    return () => {
      window.removeEventListener(QUERY_HANDOFF_APPLIED_EVENT, applyHandoff);
    };
  }, [abandonInFlight, store]);

  const messages = useMemo(() => {
    return mergeQueryMessages(
      baseMessages,
      stream.optimisticUserMessage,
      stream.pendingMessage,
    );
  }, [baseMessages, stream.optimisticUserMessage, stream.pendingMessage]);

  const { scrollRef, scrollAnchorRef, showJumpPill, jumpToLatest } =
    useStickToBottom(stream.streamingState, messages);

  const images = useQueryImages();
  const searchImage = useQuerySearchImage();

  // Drain queue when stream completes
  useEffect(() => {
    if (
      stream.queuedMessage &&
      !stream.isStreamingOrLoading &&
      stream.streamingState !== "error"
    ) {
      const next = stream.queuedMessage;
      stream.clearQueue();
      void stream.submitQuery(next);
    }
  }, [
    stream.queuedMessage,
    stream.isStreamingOrLoading,
    stream.streamingState,
    stream.clearQueue,
    stream.submitQuery,
  ]);

  const handleInputChange = useCallback((value: string) => {
    setInput(value);
  }, []);

  const handleEditMessage = useCallback(
    (messageId: string) => {
      const text = stream.beginEditFromMessage(messageId);
      if (!text) return;
      setInput(text);
      requestAnimationFrame(() => inputRef.current?.focus());
    },
    [stream],
  );

  const handleSubmit = async () => {
    const pendingSearchImage = searchImage.image?.file;
    if ((!input.trim() && !pendingSearchImage) || stream.isStreamingOrLoading) return;

    endQueryHandoffNewConversation();
    // WHY `message` may be empty while `display` is not: see `searchImageMessages`. The
    // display label must never become the text query, or it dilutes the image vector.
    const { message: queryText, display } = searchImageMessages(
      input,
      Boolean(pendingSearchImage),
      t("query.searchByImage.defaultMessage", "Find matches for this image"),
    );
    const currentImages = images.getPayload();
    setInput("");
    try {
      localStorage.removeItem(draftKeyRef.current);
    } catch {
      /* ignore */
    }
    images.clearImages();
    searchImage.clear();

    if (inputRef.current) {
      inputRef.current.style.height = "auto";
    }

    await stream.submitQuery(queryText, currentImages, pendingSearchImage, display);
  };

  const handleSuggestionClick = useCallback((text: string) => {
    setInput(text);
    inputRef.current?.focus();
  }, []);

  const handleNewConversation = useCallback(() => {
    beginNewQueryConversation();
    store.setActiveConversation(null);
    stream.setPendingMessage(null);
    stream.setOptimisticUserMessage(null);
    stream.clearEditAnchor();
    setInput("");
    try {
      localStorage.removeItem("query-draft:new");
    } catch {
      /* ignore */
    }
  }, [store, stream]);

  const isLoading = stream.isStreamingOrLoading || isLoadingConversation;

  return {
    input,
    streamingState: stream.streamingState,
    stage: stream.stage,
    stageDetail: stream.stageDetail,
    pendingMessage: stream.pendingMessage,
    messages,
    isLoading,
    isStreaming: stream.isStreamingOrLoading,
    querySettings,
    setQuerySettings,
    scrollRef,
    scrollAnchorRef,
    showJumpPill,
    jumpToLatest,
    inputRef,
    handleInputChange,
    handleSubmit,
    handleStop: stream.handleStop,
    handleRegenerate: stream.handleRegenerate,
    handleRetry: stream.handleRetry,
    handleSuggestionClick,
    handleEditMessage,
    handleNewConversation,
    queuedMessage: stream.queuedMessage,
    queueMessage: stream.queueMessage,
    clearQueue: stream.clearQueue,
    ...images,
    searchImage,
  };
}

"use client";

import type { QueryMessage, StreamingState } from "@/lib/query/query-interface-types";
import { useEffect, useRef, useState } from "react";

/** Auto-scroll behavior for the query message list (SPEC-017 UI-P3-006). */
export function useQueryScroll(
  streamingState: StreamingState,
  messages: QueryMessage[],
) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const scrollAnchorRef = useRef<HTMLDivElement>(null);
  const [shouldAutoScroll, setShouldAutoScroll] = useState(true);

  useEffect(() => {
    if (!shouldAutoScroll) return;
    scrollAnchorRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
  }, [messages, streamingState, shouldAutoScroll]);

  // Re-enable auto-scroll when the answer starts arriving.
  //
  // WHY during render: setting it in an effect meant the first streaming tick rendered with
  // auto-scroll still off (the user had scrolled up), so the first tokens landed below the fold
  // and only a second render pulled them into view — the flicker the effect was meant to prevent.
  // Deriving "streaming now" against the previous value flips it on the same render.
  const streamingNow = streamingState === "thinking" || streamingState === "generating";
  const [wasStreaming, setWasStreaming] = useState(false);
  if (streamingNow !== wasStreaming) {
    setWasStreaming(streamingNow);
    if (streamingNow) setShouldAutoScroll(true);
  }

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

  return { scrollRef, scrollAnchorRef };
}

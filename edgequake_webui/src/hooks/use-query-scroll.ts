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

  return { scrollRef, scrollAnchorRef };
}

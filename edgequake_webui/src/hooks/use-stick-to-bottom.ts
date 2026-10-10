"use client";

import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type RefObject,
} from "react";
import type { StreamingState } from "@/lib/query/query-interface-types";

const NEAR_BOTTOM_PX = 100;

/**
 * Stick-to-bottom scroll for chat streams (SPEC-155 Q11).
 * - Instant scroll (no smooth per-token jitter)
 * - rAF-batched
 * - Does NOT re-force stickiness when user scrolls up mid-stream
 * - Exposes jumpToLatest + showJumpPill
 */
export function useStickToBottom(
  streamingState: StreamingState,
  dependency: unknown,
) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const scrollAnchorRef = useRef<HTMLDivElement>(null);
  const [shouldAutoScroll, setShouldAutoScroll] = useState(true);
  const [showJumpPill, setShowJumpPill] = useState(false);
  const rafRef = useRef<number | null>(null);
  const userDetachedRef = useRef(false);

  const getViewport = useCallback((): HTMLElement | null => {
    return (
      (scrollRef.current?.querySelector(
        "[data-radix-scroll-area-viewport]",
      ) as HTMLElement | null) ?? scrollRef.current
    );
  }, []);

  const scrollToBottom = useCallback(
    (behavior: ScrollBehavior = "auto") => {
      const viewport = getViewport();
      if (viewport) {
        viewport.scrollTo({ top: viewport.scrollHeight, behavior });
      } else {
        scrollAnchorRef.current?.scrollIntoView({ behavior, block: "end" });
      }
    },
    [getViewport],
  );

  const jumpToLatest = useCallback(() => {
    userDetachedRef.current = false;
    setShouldAutoScroll(true);
    setShowJumpPill(false);
    scrollToBottom("auto");
  }, [scrollToBottom]);

  useEffect(() => {
    const viewport = getViewport();
    if (!viewport) return;

    const handleScroll = () => {
      const { scrollTop, scrollHeight, clientHeight } = viewport;
      const isNearBottom = scrollHeight - scrollTop - clientHeight < NEAR_BOTTOM_PX;
      if (!isNearBottom) {
        userDetachedRef.current = true;
        setShouldAutoScroll(false);
        setShowJumpPill(true);
      } else {
        userDetachedRef.current = false;
        setShouldAutoScroll(true);
        setShowJumpPill(false);
      }
    };

    const handleWheel = (event: WheelEvent) => {
      if (event.deltaY < 0) {
        userDetachedRef.current = true;
        setShouldAutoScroll(false);
        setShowJumpPill(true);
      }
    };

    viewport.addEventListener("scroll", handleScroll, { passive: true });
    viewport.addEventListener("wheel", handleWheel, { passive: true });
    return () => {
      viewport.removeEventListener("scroll", handleScroll);
      viewport.removeEventListener("wheel", handleWheel);
    };
  }, [getViewport]);

  // Only re-enable stickiness when a NEW stream starts (idle → thinking),
  // not on every generating tick (Q11).
  const prevStreamingRef = useRef(streamingState);
  useEffect(() => {
    const prev = prevStreamingRef.current;
    prevStreamingRef.current = streamingState;
    if (
      (streamingState === "thinking" || streamingState === "generating") &&
      prev === "idle"
    ) {
      userDetachedRef.current = false;
      setShouldAutoScroll(true);
      setShowJumpPill(false);
    }
  }, [streamingState]);

  useEffect(() => {
    if (!shouldAutoScroll || userDetachedRef.current) return;
    if (rafRef.current != null) cancelAnimationFrame(rafRef.current);
    rafRef.current = requestAnimationFrame(() => {
      scrollToBottom("auto");
      rafRef.current = null;
    });
    return () => {
      if (rafRef.current != null) cancelAnimationFrame(rafRef.current);
    };
  }, [dependency, streamingState, shouldAutoScroll, scrollToBottom]);

  return {
    scrollRef: scrollRef as RefObject<HTMLDivElement>,
    scrollAnchorRef: scrollAnchorRef as RefObject<HTMLDivElement>,
    showJumpPill,
    jumpToLatest,
    shouldAutoScroll,
  };
}

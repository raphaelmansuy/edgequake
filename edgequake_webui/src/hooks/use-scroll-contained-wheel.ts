"use client";

import { useCallback, useEffect, type RefObject } from "react";

/** CSS class applied to scroll containers inside portaled popovers. */
export const SCROLL_CONTAINED_LIST_CLASS =
  "overscroll-contain touch-pan-y";

/**
 * Keeps wheel events inside a popover list so parent pages (Settings ScrollArea)
 * do not steal scroll. Reusable across cmdk / custom listboxes.
 */
export function useScrollContainedWheel<T extends HTMLElement>() {
  const onWheel = useCallback((event: React.WheelEvent<T>) => {
    const element = event.currentTarget;
    const canScroll = element.scrollHeight > element.clientHeight;
    if (!canScroll) return;

    // Prevent parent ScrollArea / page from consuming wheel (Settings, Workspace).
    event.stopPropagation();

    const before = element.scrollTop;
    element.scrollTop += event.deltaY;

    // When we handled scroll, block native bubbling that would scroll the page.
    if (element.scrollTop !== before) {
      event.preventDefault();
    }
  }, []);

  return { onWheel, className: SCROLL_CONTAINED_LIST_CLASS };
}

const CMDK_SELECTED_ITEM_SELECTOR = '[cmdk-item][data-selected="true"]';

/** Scroll the cmdk-highlighted item into view after keyboard navigation. */
export function scrollSelectedCmdkItemIntoView(
  listRef: RefObject<HTMLElement | null>,
): void {
  const list = listRef.current;
  if (!list) return;
  const selected = list.querySelector(CMDK_SELECTED_ITEM_SELECTOR);
  if (selected instanceof HTMLElement) {
    selected.scrollIntoView({ block: "nearest" });
  }
}

/**
 * When cmdk selection changes (keyboard), keep the highlighted row visible.
 */
export function useScrollSelectedIntoView(
  listRef: RefObject<HTMLElement | null>,
  enabled: boolean,
): void {
  useEffect(() => {
    if (!enabled) return;
    const list = listRef.current;
    if (!list) return;

    const observer = new MutationObserver(() => {
      scrollSelectedCmdkItemIntoView(listRef);
    });

    observer.observe(list, {
      subtree: true,
      attributes: true,
      attributeFilter: ["data-selected", "aria-selected"],
    });

    return () => observer.disconnect();
  }, [enabled, listRef]);
}

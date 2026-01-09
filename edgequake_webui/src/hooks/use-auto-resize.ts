"use client";

import { RefObject, useCallback, useLayoutEffect } from "react";

/**
 * @module use-auto-resize
 * @description Hook to auto-resize a textarea based on content
 *
 * @implements FEAT0633 - Textarea auto-resize for chat input
 * @implements FEAT0634 - Adaptive input height based on content
 *
 * @enforces BR0621 - Max height prevents viewport overflow
 *
 * @param textareaRef - Reference to the textarea element
 * @param value - Current value of the textarea
 * @param minHeight - Minimum height in pixels (default: 60)
 * @param maxHeight - Maximum height in pixels (default: 200)
 * @returns A resize function that can be called manually
 *
 * @example
 * ```tsx
 * const textareaRef = useRef<HTMLTextAreaElement>(null);
 * useAutoResize(textareaRef, value, 60, 200);
 *
 * return <Textarea ref={textareaRef} value={value} className="resize-none" />;
 * ```
 */
export function useAutoResize(
  textareaRef: RefObject<HTMLTextAreaElement | null>,
  value: string,
  minHeight = 60,
  maxHeight = 200
): () => void {
  const resize = useCallback(() => {
    const textarea = textareaRef.current;
    if (!textarea) return;

    // Reset height to get accurate scrollHeight
    textarea.style.height = "auto";

    // Calculate new height with min/max bounds
    const scrollHeight = textarea.scrollHeight;
    const newHeight = Math.max(minHeight, Math.min(scrollHeight, maxHeight));

    textarea.style.height = `${newHeight}px`;

    // Enable overflow scroll when at max height
    textarea.style.overflowY = scrollHeight > maxHeight ? "auto" : "hidden";
  }, [textareaRef, minHeight, maxHeight]);

  useLayoutEffect(() => {
    resize();
  }, [value, resize]);

  return resize;
}

export default useAutoResize;

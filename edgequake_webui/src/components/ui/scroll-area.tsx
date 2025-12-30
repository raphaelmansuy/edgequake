"use client"

import * as ScrollAreaPrimitive from "@radix-ui/react-scroll-area";
import * as React from "react";

import { cn } from "@/lib/utils";

interface ScrollAreaProps extends React.ComponentProps<typeof ScrollAreaPrimitive.Root> {
  /** Show gradient shadow indicators when content is scrollable */
  showShadows?: boolean;
}

function ScrollArea({
  className,
  children,
  showShadows = false,
  ...props
}: ScrollAreaProps) {
  const [scrollState, setScrollState] = React.useState({
    atTop: true,
    atBottom: true,
  });
  const viewportRef = React.useRef<HTMLDivElement>(null);

  const updateScrollState = React.useCallback(() => {
    const viewport = viewportRef.current;
    if (!viewport) return;

    const { scrollTop, scrollHeight, clientHeight } = viewport;
    const atTop = scrollTop <= 1;
    const atBottom = scrollTop + clientHeight >= scrollHeight - 1;

    setScrollState((prev) => {
      if (prev.atTop === atTop && prev.atBottom === atBottom) return prev;
      return { atTop, atBottom };
    });
  }, []);

  React.useEffect(() => {
    const viewport = viewportRef.current;
    if (!viewport || !showShadows) return;

    // Initial check
    updateScrollState();

    // Use ResizeObserver to detect content changes
    const resizeObserver = new ResizeObserver(updateScrollState);
    resizeObserver.observe(viewport);
    
    // Also observe the first child for content changes
    if (viewport.firstElementChild) {
      resizeObserver.observe(viewport.firstElementChild);
    }

    viewport.addEventListener("scroll", updateScrollState, { passive: true });

    return () => {
      resizeObserver.disconnect();
      viewport.removeEventListener("scroll", updateScrollState);
    };
  }, [showShadows, updateScrollState]);

  return (
    <ScrollAreaPrimitive.Root
      data-slot="scroll-area"
      className={cn("relative", className)}
      {...props}
    >
      {/* Top shadow indicator */}
      {showShadows && (
        <div
          className={cn(
            "pointer-events-none absolute top-0 left-0 right-2.5 h-6 z-10",
            "bg-gradient-to-b from-background/80 to-transparent",
            "transition-opacity duration-200",
            scrollState.atTop ? "opacity-0" : "opacity-100"
          )}
          aria-hidden="true"
        />
      )}
      
      <ScrollAreaPrimitive.Viewport
        ref={viewportRef}
        data-slot="scroll-area-viewport"
        className="focus-visible:ring-ring/50 size-full rounded-[inherit] transition-[color,box-shadow] outline-none focus-visible:ring-[3px] focus-visible:outline-1"
      >
        {children}
      </ScrollAreaPrimitive.Viewport>
      
      {/* Bottom shadow indicator */}
      {showShadows && (
        <div
          className={cn(
            "pointer-events-none absolute bottom-0 left-0 right-2.5 h-6 z-10",
            "bg-gradient-to-t from-background/80 to-transparent",
            "transition-opacity duration-200",
            scrollState.atBottom ? "opacity-0" : "opacity-100"
          )}
          aria-hidden="true"
        />
      )}
      
      <ScrollBar />
      <ScrollAreaPrimitive.Corner />
    </ScrollAreaPrimitive.Root>
  )
}

function ScrollBar({
  className,
  orientation = "vertical",
  ...props
}: React.ComponentProps<typeof ScrollAreaPrimitive.ScrollAreaScrollbar>) {
  return (
    <ScrollAreaPrimitive.ScrollAreaScrollbar
      data-slot="scroll-area-scrollbar"
      orientation={orientation}
      className={cn(
        "flex touch-none p-px transition-colors select-none",
        orientation === "vertical" &&
          "h-full w-2.5 border-l border-l-transparent",
        orientation === "horizontal" &&
          "h-2.5 flex-col border-t border-t-transparent",
        className
      )}
      {...props}
    >
      <ScrollAreaPrimitive.ScrollAreaThumb
        data-slot="scroll-area-thumb"
        className="bg-border relative flex-1 rounded-full"
      />
    </ScrollAreaPrimitive.ScrollAreaScrollbar>
  )
}

export { ScrollArea, ScrollBar };


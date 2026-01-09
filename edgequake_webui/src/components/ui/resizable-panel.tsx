'use client';

import { cn } from '@/lib/utils';
import { useCallback, useEffect, useRef, useState } from 'react';

interface ResizablePanelProps {
  children: React.ReactNode;
  side: 'left' | 'right';
  defaultWidth: number;
  minWidth: number;
  maxWidth: number;
  className?: string;
  onWidthChange?: (width: number) => void;
  /** Storage key for persisting width. If provided, width is saved to localStorage. */
  storageKey?: string;
  /** ARIA label for the resize handle */
  ariaLabel?: string;
}

/**
 * A resizable panel component with a draggable handle.
 * Provides smooth resize experience with visual feedback.
 * Supports mouse, touch, and keyboard controls.
 */
export function ResizablePanel({
  children,
  side,
  defaultWidth,
  minWidth,
  maxWidth,
  className,
  onWidthChange,
  storageKey,
  ariaLabel = 'Resize panel',
}: ResizablePanelProps) {
  // Load persisted width from localStorage
  const getInitialWidth = useCallback(() => {
    if (typeof window === 'undefined' || !storageKey) return defaultWidth;
    try {
      const stored = localStorage.getItem(storageKey);
      if (stored) {
        const parsed = Number.parseInt(stored, 10);
        if (!Number.isNaN(parsed) && parsed >= minWidth && parsed <= maxWidth) {
          return parsed;
        }
      }
    } catch {
      // Ignore localStorage errors
    }
    return defaultWidth;
  }, [defaultWidth, minWidth, maxWidth, storageKey]);

  const [width, setWidth] = useState(defaultWidth);
  const [isResizing, setIsResizing] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);
  const startXRef = useRef(0);
  const startWidthRef = useRef(0);
  const initializedRef = useRef(false);

  // Initialize width from localStorage on mount
  useEffect(() => {
    if (!initializedRef.current) {
      const initial = getInitialWidth();
      // Intentional: One-time initialization from localStorage
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setWidth(initial);
      initializedRef.current = true;
    }
  }, [getInitialWidth]);

  // Persist width to localStorage when it changes
  useEffect(() => {
    if (storageKey && initializedRef.current && typeof window !== 'undefined') {
      try {
        localStorage.setItem(storageKey, width.toString());
      } catch {
        // Ignore localStorage errors
      }
    }
  }, [width, storageKey]);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);
    startXRef.current = e.clientX;
    startWidthRef.current = width;
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
  }, [width]);

  // Touch support
  const handleTouchStart = useCallback((e: React.TouchEvent) => {
    const touch = e.touches[0];
    setIsResizing(true);
    startXRef.current = touch.clientX;
    startWidthRef.current = width;
  }, [width]);

  const handleMouseMove = useCallback((e: MouseEvent) => {
    if (!isResizing) return;
    
    const delta = side === 'left' 
      ? e.clientX - startXRef.current
      : startXRef.current - e.clientX;
    
    const newWidth = Math.min(maxWidth, Math.max(minWidth, startWidthRef.current + delta));
    setWidth(newWidth);
    onWidthChange?.(newWidth);
  }, [isResizing, side, minWidth, maxWidth, onWidthChange]);

  const handleTouchMove = useCallback((e: TouchEvent) => {
    if (!isResizing) return;
    const touch = e.touches[0];
    
    const delta = side === 'left' 
      ? touch.clientX - startXRef.current
      : startXRef.current - touch.clientX;
    
    const newWidth = Math.min(maxWidth, Math.max(minWidth, startWidthRef.current + delta));
    setWidth(newWidth);
    onWidthChange?.(newWidth);
  }, [isResizing, side, minWidth, maxWidth, onWidthChange]);

  const handleEnd = useCallback(() => {
    setIsResizing(false);
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
  }, []);

  // Keyboard support for accessibility
  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    const step = e.shiftKey ? 50 : 10; // Larger step with shift
    
    if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') {
      e.preventDefault();
      const direction = side === 'left' 
        ? (e.key === 'ArrowRight' ? 1 : -1)
        : (e.key === 'ArrowLeft' ? 1 : -1);
      
      const newWidth = Math.min(maxWidth, Math.max(minWidth, width + (step * direction)));
      setWidth(newWidth);
      onWidthChange?.(newWidth);
    } else if (e.key === 'Home') {
      e.preventDefault();
      setWidth(minWidth);
      onWidthChange?.(minWidth);
    } else if (e.key === 'End') {
      e.preventDefault();
      setWidth(maxWidth);
      onWidthChange?.(maxWidth);
    }
  }, [side, minWidth, maxWidth, width, onWidthChange]);

  useEffect(() => {
    if (isResizing) {
      window.addEventListener('mousemove', handleMouseMove);
      window.addEventListener('mouseup', handleEnd);
      window.addEventListener('touchmove', handleTouchMove, { passive: false });
      window.addEventListener('touchend', handleEnd);
      return () => {
        window.removeEventListener('mousemove', handleMouseMove);
        window.removeEventListener('mouseup', handleEnd);
        window.removeEventListener('touchmove', handleTouchMove);
        window.removeEventListener('touchend', handleEnd);
      };
    }
  }, [isResizing, handleMouseMove, handleTouchMove, handleEnd]);

  return (
    <div
      ref={panelRef}
      className={cn('relative flex shrink-0', className)}
      style={{ width }}
    >
      {/* Resize Handle */}
      <div
        role="separator"
        aria-orientation="vertical"
        aria-valuenow={width}
        aria-valuemin={minWidth}
        aria-valuemax={maxWidth}
        aria-label={ariaLabel}
        tabIndex={0}
        className={cn(
          'absolute top-0 bottom-0 w-2 z-10 cursor-col-resize group',
          'transition-colors duration-150 focus-visible:outline-none',
          side === 'left' ? '-right-1' : '-left-1',
          isResizing ? 'bg-primary/20' : 'hover:bg-primary/10'
        )}
        onMouseDown={handleMouseDown}
        onTouchStart={handleTouchStart}
        onKeyDown={handleKeyDown}
      >
        {/* Visual indicator line */}
        <div 
          className={cn(
            'absolute top-0 bottom-0 w-0.5',
            side === 'left' ? 'right-0.5' : 'left-0.5',
            isResizing ? 'bg-primary' : 'bg-border group-hover:bg-primary/50 group-focus-visible:bg-primary'
          )}
        />
        {/* Drag handle indicator */}
        <div 
          className={cn(
            'absolute top-1/2 -translate-y-1/2 w-1 h-12 rounded-full',
            'opacity-0 group-hover:opacity-100 group-focus-visible:opacity-100 transition-opacity',
            side === 'left' ? 'right-0' : 'left-0',
            isResizing ? 'opacity-100 bg-primary' : 'bg-muted-foreground/30'
          )}
        />
      </div>
      
      {/* Panel Content */}
      <div className="flex-1 overflow-hidden">
        {children}
      </div>
    </div>
  );
}

export default ResizablePanel;

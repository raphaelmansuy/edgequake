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
}

/**
 * A resizable panel component with a draggable handle.
 * Provides smooth resize experience with visual feedback.
 */
export function ResizablePanel({
  children,
  side,
  defaultWidth,
  minWidth,
  maxWidth,
  className,
  onWidthChange,
}: ResizablePanelProps) {
  const [width, setWidth] = useState(defaultWidth);
  const [isResizing, setIsResizing] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);
  const startXRef = useRef(0);
  const startWidthRef = useRef(0);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);
    startXRef.current = e.clientX;
    startWidthRef.current = width;
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
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

  const handleMouseUp = useCallback(() => {
    setIsResizing(false);
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
  }, []);

  useEffect(() => {
    if (isResizing) {
      window.addEventListener('mousemove', handleMouseMove);
      window.addEventListener('mouseup', handleMouseUp);
      return () => {
        window.removeEventListener('mousemove', handleMouseMove);
        window.removeEventListener('mouseup', handleMouseUp);
      };
    }
  }, [isResizing, handleMouseMove, handleMouseUp]);

  return (
    <div
      ref={panelRef}
      className={cn('relative flex shrink-0', className)}
      style={{ width }}
    >
      {/* Resize Handle */}
      <div
        className={cn(
          'absolute top-0 bottom-0 w-1 z-10 cursor-col-resize group',
          'transition-colors duration-150',
          side === 'left' ? 'right-0' : 'left-0',
          isResizing ? 'bg-primary' : 'hover:bg-primary/50'
        )}
        onMouseDown={handleMouseDown}
      >
        {/* Visual indicator */}
        <div 
          className={cn(
            'absolute top-1/2 -translate-y-1/2 w-1 h-12 rounded-full',
            'opacity-0 group-hover:opacity-100 transition-opacity',
            side === 'left' ? '-right-0.5' : '-left-0.5',
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

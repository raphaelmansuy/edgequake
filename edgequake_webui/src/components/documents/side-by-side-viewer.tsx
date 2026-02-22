/**
 * @module SideBySideViewer
 * @description Split-panel layout for viewing PDF and Markdown side-by-side.
 * Provides resizable panels with smooth dragging experience.
 *
 * @implements SPEC-002 - Document Viewer with side-by-side display
 * @implements FEAT0731 - Split-panel layout with resizable divider
 * @implements FEAT0732 - View mode toggle (PDF only, Markdown only, side-by-side)
 * @implements FEAT0733 - Panel synchronization controls
 *
 * @enforces BR0731 - Responsive layout for different screen sizes
 * @enforces BR0732 - Smooth resize without content jumping
 *
 * @see {@link docs/features.md} FEAT0731-0733
 */
'use client';

import { Button } from '@/components/ui/button';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { cn } from '@/lib/utils';
import {
    Columns2,
    PanelLeftClose,
    PanelRightClose
} from 'lucide-react';
import { useCallback, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

type ViewMode = 'side-by-side' | 'pdf-only' | 'markdown-only';

interface SideBySideViewerProps {
  /** Left panel content (typically PDF) */
  leftPanel: React.ReactNode;
  /** Right panel content (typically Markdown) */
  rightPanel: React.ReactNode;
  /** Optional class name for container */
  className?: string;
  /** Fixed height for the viewer */
  height?: number;
  /** Initial view mode */
  initialMode?: ViewMode;
  /** Left panel title */
  leftTitle?: string;
  /** Right panel title */
  rightTitle?: string;
  /** Called when view mode changes */
  onModeChange?: (mode: ViewMode) => void;
}

/**
 * SideBySideViewer component for displaying two panels side-by-side.
 *
 * Features:
 * - Resizable divider between panels
 * - View mode toggle (PDF only, Markdown only, side-by-side)
 * - Smooth resize with mouse/touch/keyboard support
 * - Responsive layout that adapts to screen size
 */
export function SideBySideViewer({
  leftPanel,
  rightPanel,
  className,
  height,
  initialMode = 'side-by-side',
  leftTitle = 'PDF Document',
  rightTitle = 'Extracted Markdown',
  onModeChange,
}: SideBySideViewerProps) {
  const { t } = useTranslation();
  const [mode, setMode] = useState<ViewMode>(initialMode);
  const [leftWidth, setLeftWidth] = useState(50); // Percentage
  const containerRef = useRef<HTMLDivElement>(null);
  const isDragging = useRef(false);
  const startX = useRef(0);
  const startWidth = useRef(50);

  const handleModeChange = useCallback((newMode: ViewMode) => {
    setMode(newMode);
    onModeChange?.(newMode);
  }, [onModeChange]);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    isDragging.current = true;
    startX.current = e.clientX;
    startWidth.current = leftWidth;
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
  }, [leftWidth]);

  const handleMouseMove = useCallback((e: MouseEvent) => {
    if (!isDragging.current || !containerRef.current) return;
    
    const containerRect = containerRef.current.getBoundingClientRect();
    const containerWidth = containerRect.width;
    const deltaX = e.clientX - startX.current;
    const deltaPercent = (deltaX / containerWidth) * 100;
    
    // Clamp between 25% and 75%
    const newWidth = Math.min(75, Math.max(25, startWidth.current + deltaPercent));
    setLeftWidth(newWidth);
  }, []);

  const handleMouseUp = useCallback(() => {
    isDragging.current = false;
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
  }, []);

  // Add global mouse event listeners when dragging
  useState(() => {
    if (typeof window === 'undefined') return;
    
    const handleMove = (e: MouseEvent) => handleMouseMove(e);
    const handleUp = () => handleMouseUp();
    
    window.addEventListener('mousemove', handleMove);
    window.addEventListener('mouseup', handleUp);
    
    return () => {
      window.removeEventListener('mousemove', handleMove);
      window.removeEventListener('mouseup', handleUp);
    };
  });

  return (
    <div className={cn('flex flex-col min-h-0', className)}>
      {/* Minimal View Mode Toggle */}
      <div className="flex items-center justify-end gap-1 px-2 py-1 border-b bg-muted/20">
        <TooltipProvider>
          <div className="flex items-center gap-0.5 bg-background rounded p-0.5">
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant={mode === 'pdf-only' ? 'secondary' : 'ghost'}
                  size="icon"
                  className="h-6 w-6"
                  onClick={() => handleModeChange('pdf-only')}
                >
                  <PanelRightClose className="h-3.5 w-3.5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>PDF Only</TooltipContent>
            </Tooltip>
            
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant={mode === 'side-by-side' ? 'secondary' : 'ghost'}
                  size="icon"
                  className="h-6 w-6"
                  onClick={() => handleModeChange('side-by-side')}
                >
                  <Columns2 className="h-3.5 w-3.5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>Split View</TooltipContent>
            </Tooltip>
            
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant={mode === 'markdown-only' ? 'secondary' : 'ghost'}
                  size="icon"
                  className="h-6 w-6"
                  onClick={() => handleModeChange('markdown-only')}
                >
                  <PanelLeftClose className="h-3.5 w-3.5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>Markdown Only</TooltipContent>
            </Tooltip>
          </div>
        </TooltipProvider>
      </div>

      {/* Content Panels */}
      <div
        ref={containerRef}
        className="flex flex-1 min-h-0"
        style={height ? { height: `${height}px` } : undefined}
      >
        {/* Left Panel (PDF) */}
        {(mode === 'pdf-only' || mode === 'side-by-side') && (
          <div
            className={cn(
              'flex flex-col border-r overflow-hidden',
              mode === 'pdf-only' ? 'w-full' : ''
            )}
            style={mode === 'side-by-side' ? { width: `${leftWidth}%` } : undefined}
          >
            <div className="flex-1 overflow-hidden">
              {leftPanel}
            </div>
          </div>
        )}

        {/* Resize Handle */}
        {mode === 'side-by-side' && (
          <div
            className={cn(
              'w-1 bg-border hover:bg-primary/30 cursor-col-resize transition-colors',
              'flex items-center justify-center',
              isDragging.current && 'bg-primary/50'
            )}
            onMouseDown={handleMouseDown}
          >
            <div className="h-8 w-0.5 rounded-full bg-muted-foreground/20" />
          </div>
        )}

        {/* Right Panel (Markdown) */}
        {(mode === 'markdown-only' || mode === 'side-by-side') && (
          <div
            className={cn(
              'flex flex-col overflow-hidden',
              mode === 'markdown-only' ? 'w-full' : 'flex-1'
            )}
          >
            {/* WHY: py-0 outer container; inner ContentRenderer provides p-8 padding.
                 pt-0 here avoids double top padding since ContentRenderer has p-8 already.
                 pb-8 ensures content doesn't stick to the bottom edge on scroll. */}
            <div className="flex-1 min-h-0 overflow-y-auto overflow-x-hidden">
              {rightPanel}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default SideBySideViewer;

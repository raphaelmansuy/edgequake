/**
 * @module RightPanel
 * @description Reusable collapsible right panel for detail views.
 * Used for entity details, document preview, and settings panels.
 *
 * @implements FEAT0614 - Collapsible detail panels
 * @implements FEAT0615 - Configurable panel widths
 * @implements FEAT0616 - Scroll area for long content
 * @implements FEAT0617 - Resizable panels with localStorage persistence
 *
 * @enforces BR0201 - Panel syncs with main view selection
 * @enforces BR0610 - Panel state persists during session
 *
 * @see {@link docs/features.md} FEAT0614-0617
 */
'use client';

import { Button } from '@/components/ui/button';
import { ResizablePanel } from '@/components/ui/resizable-panel';
import { ScrollArea } from '@/components/ui/scroll-area';
import { cn } from '@/lib/utils';
import { ChevronLeft, ChevronRight, X } from 'lucide-react';
import { forwardRef, type ReactNode } from 'react';

interface RightPanelProps {
  /** Whether the panel is currently open/expanded */
  isOpen: boolean;
  /** Callback when the panel should be toggled */
  onToggle: () => void;
  /** Callback when the panel should be closed */
  onClose?: () => void;
  /** Panel title displayed in the header */
  title?: string;
  /** Panel subtitle/description */
  subtitle?: string;
  /** Panel width when expanded - 'narrow' (320px) or 'wide' (400px) - only used if not resizable */
  width?: 'narrow' | 'wide';
  /** Content to render inside the panel */
  children: ReactNode;
  /** Additional class names for the container */
  className?: string;
  /** Show a collapsed indicator bar when closed */
  showCollapsedBar?: boolean;
  /** Label to show on the collapsed bar */
  collapsedLabel?: string;
  /** Icon to show in the header */
  headerIcon?: ReactNode;

  // Resizable panel props
  /** Whether the panel can be resized */
  resizable?: boolean;
  /** Default width for resizable panel in pixels */
  defaultWidth?: number;
  /** Minimum width for resizable panel in pixels */
  minWidth?: number;
  /** Maximum width for resizable panel in pixels */
  maxWidth?: number;
  /** Storage key for persisting panel width */
  storageKey?: string;
}

/**
 * Reusable right panel component for consistent panel behavior across the application.
 * Features:
 * - Collapsible with smooth animation
 * - Configurable width (narrow: 320px, wide: 400px) or fully resizable
 * - Optional collapsed indicator bar
 * - Scroll area for content
 * - Optional resize with localStorage persistence
 */
export const RightPanel = forwardRef<HTMLDivElement, RightPanelProps>(
  function RightPanel(
    {
      isOpen,
      onToggle,
      onClose,
      title,
      subtitle,
      width = 'wide',
      children,
      className,
      showCollapsedBar = true,
      collapsedLabel,
      headerIcon,
      resizable = false,
      defaultWidth = 400,
      minWidth = 320,
      maxWidth = 800,
      storageKey,
    },
    ref
  ) {
    const panelWidth = width === 'narrow' ? 'w-80' : 'w-[400px]';

    // When collapsed, show a thin bar that can be clicked to expand
    if (!isOpen && showCollapsedBar) {
      return (
        <div
          ref={ref}
          className={cn(
            "w-10 border-l bg-card/50 flex flex-col items-center py-4 cursor-pointer hover:bg-muted transition-colors",
            className
          )}
          onClick={onToggle}
          role="button"
          tabIndex={0}
          aria-label={`Expand ${collapsedLabel || 'panel'}`}
          onKeyDown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              onToggle();
            }
          }}
        >
          <ChevronLeft className="h-4 w-4 text-muted-foreground mb-2" />
          {collapsedLabel && (
            <span
              className="text-xs text-muted-foreground writing-mode-vertical"
              style={{ writingMode: 'vertical-rl', transform: 'rotate(180deg)' }}
            >
              {collapsedLabel}
            </span>
          )}
        </div>
      );
    }

    if (!isOpen) {
      return null;
    }

    const panelContent = (
      <aside
        ref={ref}
        className={cn(
          resizable ? 'w-full' : panelWidth,
          // WHY: h-full constrains the aside to its container height so the inner
          // ScrollArea (flex-1 min-h-0) can scroll instead of the aside growing
          // to fit all content and being clipped by overflow-hidden on the parent.
          "border-l bg-card flex flex-col h-full transition-all duration-300 ease-in-out overflow-hidden",
          className
        )}
        aria-label={title || 'Side panel'}
      >
        {/* Header */}
        {(title || onClose) && (
          <div className="flex items-center justify-between border-b px-3 py-2 flex-shrink-0 bg-muted/20">
            <div className="flex items-center gap-2 min-w-0">
              {headerIcon && (
                <div className="flex-shrink-0 text-muted-foreground">
                  {headerIcon}
                </div>
              )}
              <div className="min-w-0">
                {title && (
                  <h3 className="text-xs font-semibold truncate">{title}</h3>
                )}
                {subtitle && (
                  <p className="text-[10px] text-muted-foreground truncate">{subtitle}</p>
                )}
              </div>
            </div>
            <div className="flex items-center gap-0.5 flex-shrink-0">
              <Button
                variant="ghost"
                size="icon"
                className="h-6 w-6"
                onClick={onToggle}
                aria-label="Collapse panel"
              >
                <ChevronRight className="h-3.5 w-3.5" />
              </Button>
              {onClose && (
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-6 w-6"
                  onClick={onClose}
                  aria-label="Close panel"
                >
                  <X className="h-3.5 w-3.5" />
                </Button>
              )}
            </div>
          </div>
        )}

        {/* Content
             WHY: pb-8 ensures bottom items clear the bottom gradient shadow (h-6 = 24px)
             and provides breathing room at the end of the panel. */}
        <ScrollArea className="flex-1 min-h-0" showShadows>
          <div className="px-4 pt-4 pb-8">{children}</div>
        </ScrollArea>
      </aside>
    );

    // If resizable, wrap with ResizablePanel
    if (resizable) {
      return (
        <ResizablePanel
          side="right"
          defaultWidth={defaultWidth}
          minWidth={minWidth}
          maxWidth={maxWidth}
          storageKey={storageKey}
          ariaLabel={`Resize ${title || 'panel'}`}
        >
          {panelContent}
        </ResizablePanel>
      );
    }

    return panelContent;
  }
);

export default RightPanel;

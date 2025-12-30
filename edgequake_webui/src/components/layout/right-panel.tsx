'use client';

import { Button } from '@/components/ui/button';
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
  /** Panel width when expanded - 'narrow' (320px) or 'wide' (400px) */
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
}

/**
 * Reusable right panel component for consistent panel behavior across the application.
 * Features:
 * - Collapsible with smooth animation
 * - Configurable width (narrow: 320px, wide: 400px)
 * - Optional collapsed indicator bar
 * - Scroll area for content
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

    return (
      <aside
        ref={ref}
        className={cn(
          panelWidth,
          "border-l bg-card flex flex-col transition-all duration-300 ease-in-out overflow-hidden",
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

        {/* Content */}
        <ScrollArea className="flex-1 min-h-0" showShadows>
          <div className="p-4">{children}</div>
        </ScrollArea>
      </aside>
    );
  }
);

export default RightPanel;

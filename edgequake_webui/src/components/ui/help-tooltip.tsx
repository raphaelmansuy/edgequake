'use client';

import {
    Tooltip,
    TooltipContent,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { cn } from '@/lib/utils';
import { HelpCircle, Keyboard } from 'lucide-react';
import * as React from 'react';

interface HelpTooltipProps {
  /** The help text to display */
  content: React.ReactNode;
  /** Optional keyboard shortcut to display */
  shortcut?: string | string[];
  /** Side to display the tooltip */
  side?: 'top' | 'right' | 'bottom' | 'left';
  /** Additional className for the trigger */
  className?: string;
  /** Size of the help icon */
  size?: 'sm' | 'md' | 'lg';
  /** Whether the trigger is just a small icon inline with text */
  inline?: boolean;
  /** Custom trigger element (overrides default help icon) */
  children?: React.ReactNode;
}

/**
 * A contextual help tooltip component with a consistent style.
 * Displays a help icon that shows helpful information on hover/focus.
 */
export function HelpTooltip({
  content,
  shortcut,
  side = 'top',
  className,
  size = 'sm',
  inline = false,
  children,
}: HelpTooltipProps) {
  const sizeClasses = {
    sm: 'h-3.5 w-3.5',
    md: 'h-4 w-4',
    lg: 'h-5 w-5',
  };

  const shortcuts = Array.isArray(shortcut) ? shortcut : shortcut ? [shortcut] : [];

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        {children ? (
          <span className={className}>{children}</span>
        ) : (
          <button
            type="button"
            className={cn(
              'text-muted-foreground/60 hover:text-muted-foreground transition-colors rounded-full',
              'focus:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1',
              inline && 'align-middle ml-1',
              className
            )}
            aria-label="Help"
          >
            <HelpCircle className={sizeClasses[size]} />
          </button>
        )}
      </TooltipTrigger>
      <TooltipContent side={side} className="max-w-xs">
        <div className="space-y-1.5">
          <div className="text-xs leading-relaxed">{content}</div>
          {shortcuts.length > 0 && (
            <div className="flex items-center gap-1.5 pt-1 border-t border-background/20">
              <Keyboard className="h-3 w-3 opacity-60" />
              <div className="flex items-center gap-1">
                {shortcuts.map((key, index) => (
                  <React.Fragment key={key}>
                    {index > 0 && <span className="opacity-60 text-[10px]">then</span>}
                    <kbd className="px-1.5 py-0.5 text-[10px] font-mono bg-background/20 rounded border border-background/10">
                      {key}
                    </kbd>
                  </React.Fragment>
                ))}
              </div>
            </div>
          )}
        </div>
      </TooltipContent>
    </Tooltip>
  );
}

interface FeatureTooltipProps {
  /** Title of the feature */
  title: string;
  /** Description of what the feature does */
  description: string;
  /** Optional keyboard shortcuts */
  shortcuts?: Array<{ key: string; action: string }>;
  /** Side to display the tooltip */
  side?: 'top' | 'right' | 'bottom' | 'left';
  /** The element that triggers the tooltip */
  children: React.ReactNode;
  /** Additional className for the tooltip content */
  className?: string;
}

/**
 * A feature tooltip for explaining complex features with multiple shortcuts.
 * Used for controls like zoom, layout, etc.
 */
export function FeatureTooltip({
  title,
  description,
  shortcuts,
  side = 'bottom',
  children,
  className,
}: FeatureTooltipProps) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>{children}</TooltipTrigger>
      <TooltipContent side={side} className={cn('max-w-sm p-3', className)}>
        <div className="space-y-2">
          <div>
            <h4 className="font-medium text-xs mb-0.5">{title}</h4>
            <p className="text-[11px] opacity-80 leading-relaxed">{description}</p>
          </div>
          {shortcuts && shortcuts.length > 0 && (
            <div className="pt-2 border-t border-background/20">
              <div className="flex items-center gap-1.5 mb-1.5">
                <Keyboard className="h-3 w-3 opacity-60" />
                <span className="text-[10px] font-medium opacity-80">Keyboard Shortcuts</span>
              </div>
              <div className="grid gap-1">
                {shortcuts.map(({ key, action }) => (
                  <div key={key} className="flex items-center justify-between gap-4 text-[10px]">
                    <span className="opacity-70">{action}</span>
                    <kbd className="px-1.5 py-0.5 font-mono bg-background/20 rounded border border-background/10 shrink-0">
                      {key}
                    </kbd>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </TooltipContent>
    </Tooltip>
  );
}

interface ShortcutHintProps {
  /** Keyboard shortcut key */
  shortcut: string;
  /** Action description */
  action: string;
}

/**
 * A simple keyboard shortcut hint badge.
 */
export function ShortcutHint({ shortcut, action }: ShortcutHintProps) {
  return (
    <div className="inline-flex items-center gap-2 text-xs text-muted-foreground">
      <span>{action}</span>
      <kbd className="px-1.5 py-0.5 font-mono text-[10px] bg-muted rounded border">
        {shortcut}
      </kbd>
    </div>
  );
}

export default HelpTooltip;

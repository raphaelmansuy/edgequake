/**
 * Collapsible Details Block Component
 * 
 * Renders HTML <details>/<summary> elements as collapsible sections.
 * Provides smooth animations and accessible keyboard navigation.
 */
'use client';

import { cn } from '@/lib/utils';
import { ChevronRight } from 'lucide-react';
import { memo, useCallback, useEffect, useRef, useState } from 'react';

interface DetailsBlockProps {
  summary: string;
  children: React.ReactNode;
  defaultOpen?: boolean;
  className?: string;
}

/**
 * Collapsible details block with smooth animation
 */
export const DetailsBlock = memo(function DetailsBlock({
  summary,
  children,
  defaultOpen = false,
  className,
}: DetailsBlockProps) {
  const [isOpen, setIsOpen] = useState(defaultOpen);
  const contentRef = useRef<HTMLDivElement>(null);
  const [height, setHeight] = useState<number | 'auto'>(defaultOpen ? 'auto' : 0);

  // Calculate content height for smooth animation
  useEffect(() => {
    if (contentRef.current) {
      if (isOpen) {
        const contentHeight = contentRef.current.scrollHeight;
        setHeight(contentHeight);
        // After animation, set to auto for dynamic content
        const timer = setTimeout(() => setHeight('auto'), 200);
        return () => clearTimeout(timer);
      } else {
        // First set the current height, then animate to 0
        if (height === 'auto') {
          setHeight(contentRef.current.scrollHeight);
          requestAnimationFrame(() => {
            setHeight(0);
          });
        } else {
          setHeight(0);
        }
      }
    }
  }, [isOpen]);

  const toggle = useCallback(() => {
    setIsOpen(prev => !prev);
  }, []);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      toggle();
    }
  }, [toggle]);

  return (
    <div 
      className={cn(
        'my-4 rounded-lg border border-zinc-200 dark:border-zinc-700',
        'bg-zinc-50/50 dark:bg-zinc-800/50',
        'overflow-hidden',
        className
      )}
    >
      {/* Summary/trigger */}
      <button
        type="button"
        onClick={toggle}
        onKeyDown={handleKeyDown}
        className={cn(
          'flex items-center gap-2 w-full px-4 py-3',
          'text-left text-sm font-medium',
          'hover:bg-zinc-100 dark:hover:bg-zinc-700/50',
          'transition-colors duration-150',
          'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50'
        )}
        aria-expanded={isOpen}
      >
        <ChevronRight 
          className={cn(
            'h-4 w-4 text-muted-foreground transition-transform duration-200',
            isOpen && 'rotate-90'
          )}
          aria-hidden="true"
        />
        <span>{summary}</span>
      </button>
      
      {/* Collapsible content */}
      <div
        ref={contentRef}
        style={{ 
          height: typeof height === 'number' ? `${height}px` : height,
          overflow: 'hidden',
        }}
        className={cn(
          'transition-[height] duration-200 ease-out',
          !isOpen && 'pointer-events-none'
        )}
      >
        <div 
          className={cn(
            'px-4 pb-4 pt-0',
            'border-t border-zinc-200 dark:border-zinc-700'
          )}
        >
          <div className="pt-3 prose prose-sm dark:prose-invert max-w-none">
            {children}
          </div>
        </div>
      </div>
    </div>
  );
});

export default DetailsBlock;

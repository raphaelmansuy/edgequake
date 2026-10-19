/**
 * SPEC-168 — one framed strip of KPI cells separated by hairlines.
 *
 * Replaces rows of individually boxed cards: a single frame reads as one
 * glanceable band and spends less chrome. Cells are borderless; `gap-px` over
 * the border colour draws the dividers at any column count.
 */
import { cn } from '@/lib/utils';
import type { ReactNode } from 'react';

export function StatStrip({
  children,
  className,
  'aria-label': ariaLabel,
}: {
  children: ReactNode;
  className?: string;
  'aria-label'?: string;
}) {
  return (
    <section
      aria-label={ariaLabel}
      className={cn(
        'grid min-w-0 grid-cols-2 gap-px overflow-hidden rounded-lg border bg-border lg:grid-cols-4 [&>*]:min-w-0',
        className,
      )}
    >
      {children}
    </section>
  );
}

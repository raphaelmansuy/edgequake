/**
 * @module GraphLoadingOverlay
 * @description Rich loading indicator for knowledge graph visualization.
 * Shows animated graph skeleton, elapsed timer, and contextual tips
 * so users know the system is working during multi-second loads.
 *
 * @implements FEAT0601 - Knowledge Graph Visualization (loading UX)
 * @enforces BR0602 - Streaming indicator for progressive loading
 */
'use client';

import { Network } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';

const LOADING_TIPS = [
  'Querying knowledge graph nodes...',
  'Calculating entity connections...',
  'Building graph layout...',
  'Fetching relationship edges...',
  'Ranking nodes by importance...',
];

interface GraphLoadingOverlayProps {
  /** Whether the overlay is visible */
  visible: boolean;
  /** Optional phase text override */
  phase?: string;
}

/** Hook: elapsed time in ms since `active` became true. Resets on deactivate. */
function useElapsedTimer(active: boolean) {
  const [elapsed, setElapsed] = useState(0);
  const startRef = useRef(0);

  useEffect(() => {
    if (!active) return;
    startRef.current = performance.now();
    // WHY: Only set state inside interval callback, not synchronously in effect body.
    // The first tick at 50ms will reset elapsed to ~0 naturally.
    const id = setInterval(() => {
      setElapsed(performance.now() - startRef.current);
    }, 100);
    return () => {
      clearInterval(id);
      setElapsed(0);
    };
  }, [active]);

  return elapsed;
}

/** Hook: rotating index that advances every `intervalMs` while `active`. */
function useRotatingTip(active: boolean, count: number, intervalMs = 3000) {
  const [index, setIndex] = useState(0);

  useEffect(() => {
    if (!active) return;
    const id = setInterval(() => setIndex((prev) => (prev + 1) % count), intervalMs);
    return () => clearInterval(id);
  }, [active, count, intervalMs]);

  return index;
}

/**
 * Rich loading overlay with animated graph skeleton, elapsed timer, and rotating tips.
 * WHY: A simple spinner gives no feedback on multi-second loads (3-15s).
 * Users need to see progress/activity to know the system hasn't frozen.
 */
export function GraphLoadingOverlay({ visible, phase }: GraphLoadingOverlayProps) {
  const elapsed = useElapsedTimer(visible);
  const tipIndex = useRotatingTip(visible, LOADING_TIPS.length);

  if (!visible) return null;

  const seconds = (elapsed / 1000).toFixed(1);

  return (
    <div className="absolute inset-0 z-30 flex items-center justify-center bg-background/80 backdrop-blur-sm">
      <div className="flex flex-col items-center gap-4 max-w-sm text-center px-6">
        {/* Animated graph icon */}
        <div className="relative">
          <div className="absolute inset-0 animate-ping opacity-20 rounded-full bg-primary" />
          <div className="relative flex items-center justify-center w-16 h-16 rounded-full bg-primary/10 border border-primary/20">
            <Network className="h-7 w-7 text-primary animate-pulse" />
          </div>
        </div>

        {/* Title */}
        <div>
          <h3 className="text-base font-semibold">Loading Knowledge Graph</h3>
          <p className="text-sm text-muted-foreground mt-1 min-h-5 transition-opacity duration-300">
            {phase || LOADING_TIPS[tipIndex]}
          </p>
        </div>

        {/* Progress bar (indeterminate) */}
        <div className="w-48 h-1.5 bg-muted rounded-full overflow-hidden">
          <div className="h-full bg-primary rounded-full animate-loading-bar" />
        </div>

        {/* Elapsed timer */}
        <p className="text-xs text-muted-foreground font-mono tabular-nums">
          {seconds}s elapsed
        </p>

        {/* Skeleton node dots — gives visual sense of graph building */}
        <div className="flex items-center gap-3 mt-2 opacity-60">
          {[0, 1, 2, 3, 4].map((i) => (
            <div
              key={i}
              className="w-3 h-3 rounded-full bg-primary/40 animate-pulse"
              style={{ animationDelay: `${i * 200}ms` }}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

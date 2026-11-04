/**
 * SPEC-143 — Shared active-page controller for PDF ↔ Markdown sync.
 *
 * SRP: owns activePage + syncMode + driver lock only.
 * Does not render panes or write the URL (parent owns router).
 *
 * Directional mode (LAW-143-9):
 * - none: panes scroll independently; URL is not driven by scroll.
 * - pdf-to-md: PDF publishes; markdown follows.
 * - md-to-pdf: markdown publishes; PDF follows.
 *
 * Lock model:
 * - User gesture on a pane (`beginGesture`) holds that driver until `endGesture`
 *   (or settleMs fallback). Cross-driver updates are ignored while held.
 * - Same-page updates do not reset the driver / lock.
 * - External (URL) updates that already match activePage are ignored (echo).
 */

'use client';

import {
  DEFAULT_PAGE_SYNC_MODE,
  followMarkdown as modeFollowsMarkdown,
  followPdf as modeFollowsPdf,
  isPageSyncMode,
  publishesFromMd,
  publishesFromPdf,
  readStoredPageSyncMode,
  writeStoredPageSyncMode,
  type PageSyncMode,
} from '@/lib/documents/page-sync-mode';
import { useCallback, useEffect, useRef, useState } from 'react';

export type { PageSyncMode };
export type PageSyncDriver = 'none' | 'pdf' | 'md' | 'external';

export interface UsePageSyncControllerOptions {
  /** Seed page (1-indexed). */
  initialPage?: number;
  /** Fallback lock window after a driver update (ms). Default 250. */
  settleMs?: number;
  /**
   * Initial sync mode. When omitted, reads `eq-page-sync-mode` from
   * localStorage (first visit → pdf-to-md).
   */
  initialSyncMode?: PageSyncMode;
  /**
   * When false, skip localStorage read/write (tests). Default true.
   */
  persistMode?: boolean;
}

export interface PageSyncController {
  activePage: number;
  syncMode: PageSyncMode;
  /** Markdown follows PDF (pdf-to-md). */
  followMarkdown: boolean;
  /** PDF follows markdown (md-to-pdf). */
  followPdf: boolean;
  /** True when a direction is selected (not none). */
  syncEnabled: boolean;
  driver: PageSyncDriver;
  setSyncMode: (mode: PageSyncMode) => void;
  setPageFromPdf: (page: number) => void;
  setPageFromMd: (page: number) => void;
  setPageFromExternal: (page: number) => void;
  /** Mark a pane as the active user-gesture driver. */
  beginGesture: (source: 'pdf' | 'md') => void;
  /** Release the gesture hold (scrollend / pointerup). */
  endGesture: () => void;
}

function clampPage(page: number): number {
  if (!Number.isFinite(page)) return 1;
  return Math.max(1, Math.floor(page));
}

/** Pure gate used by unit tests (U-143-02). */
export function shouldAcceptPageUpdate(args: {
  now: number;
  lockUntil: number;
  currentDriver: PageSyncDriver;
  source: PageSyncDriver;
  currentPage: number;
  nextPage: number;
  gestureDriver: PageSyncDriver;
}): boolean {
  if (args.nextPage === args.currentPage && args.source !== 'none') {
    // Same page: accept only to keep driver stable for non-external; external echo ignored by caller.
    return false;
  }
  if (
    args.gestureDriver !== 'none' &&
    args.gestureDriver !== args.source &&
    args.source !== 'none'
  ) {
    return false;
  }
  const locked = args.now < args.lockUntil;
  if (locked && args.currentDriver !== 'none' && args.currentDriver !== args.source) {
    return false;
  }
  return true;
}

export function usePageSyncController(
  options: UsePageSyncControllerOptions = {},
): PageSyncController {
  const settleMs = options.settleMs ?? 250;
  const persistMode = options.persistMode !== false;
  const [activePage, setActivePage] = useState(() =>
    clampPage(options.initialPage ?? 1),
  );
  // SSR has no localStorage — seed DEFAULT. Client mounts (SPA) can read storage
  // in the initializer; SSR→hydrate catches up in the effect below.
  const [syncMode, setSyncModeState] = useState<PageSyncMode>(() => {
    if (options.initialSyncMode != null && isPageSyncMode(options.initialSyncMode)) {
      return options.initialSyncMode;
    }
    if (typeof window === 'undefined' || options.persistMode === false) {
      return DEFAULT_PAGE_SYNC_MODE;
    }
    return readStoredPageSyncMode();
  });
  const [driver, setDriver] = useState<PageSyncDriver>('none');
  const lockUntilRef = useRef(0);
  const driverRef = useRef<PageSyncDriver>('none');
  const pageRef = useRef(clampPage(options.initialPage ?? 1));
  /** Last page reported by each pane (even when that pane is not the publisher). */
  const pdfPageRef = useRef(clampPage(options.initialPage ?? 1));
  const mdPageRef = useRef(clampPage(options.initialPage ?? 1));
  const syncModeRef = useRef(syncMode);
  // Mirror the prop for use inside event handlers, which run outside render. Assigning here is
  // what the `refs` rule forbids; an effect is the documented way and runs before any event.
  useEffect(() => {
    syncModeRef.current = syncMode;
  }, [syncMode]);
  const gestureDriverRef = useRef<PageSyncDriver>('none');
  const gestureTimerRef = useRef<number | null>(null);
  const hydratedModeRef = useRef(false);

  useEffect(() => {
    if (!persistMode || hydratedModeRef.current) return;
    hydratedModeRef.current = true;
    if (options.initialSyncMode != null && isPageSyncMode(options.initialSyncMode)) {
      return;
    }
    const stored = readStoredPageSyncMode();
    if (stored === syncModeRef.current) return;
    syncModeRef.current = stored;
    setSyncModeState(stored);
  }, [persistMode, options.initialSyncMode]);

  const apply = useCallback(
    (source: PageSyncDriver, page: number) => {
      const next = clampPage(page);
      const now = Date.now();
      if (
        !shouldAcceptPageUpdate({
          now,
          lockUntil: lockUntilRef.current,
          currentDriver: driverRef.current,
          source,
          currentPage: pageRef.current,
          nextPage: next,
          gestureDriver: gestureDriverRef.current,
        })
      ) {
        return;
      }
      pageRef.current = next;
      setActivePage(next);
      driverRef.current = source;
      setDriver(source);
      lockUntilRef.current = now + settleMs;
    },
    [settleMs],
  );

  const setPageFromPdf = useCallback(
    (page: number) => {
      const next = clampPage(page);
      pdfPageRef.current = next;
      if (!publishesFromPdf(syncModeRef.current)) return;
      apply('pdf', next);
    },
    [apply],
  );

  const setPageFromMd = useCallback(
    (page: number) => {
      const next = clampPage(page);
      mdPageRef.current = next;
      if (!publishesFromMd(syncModeRef.current)) return;
      apply('md', next);
    },
    [apply],
  );

  const setPageFromExternal = useCallback(
    (page: number) => {
      const next = clampPage(page);
      // URL echo: writing ?page=N must not re-lock / re-drive when already on N.
      if (next === pageRef.current) return;
      pdfPageRef.current = next;
      mdPageRef.current = next;
      apply('external', next);
    },
    [apply],
  );

  const setSyncMode = useCallback(
    (mode: PageSyncMode) => {
      if (!isPageSyncMode(mode)) return;
      const prev = syncModeRef.current;
      syncModeRef.current = mode;
      setSyncModeState(mode);
      if (persistMode) writeStoredPageSyncMode(mode);
      if (mode === prev) return;
      // Align follower once to the source pane's last reported page.
      if (mode === 'pdf-to-md') {
        apply('pdf', pdfPageRef.current);
      } else if (mode === 'md-to-pdf') {
        apply('md', mdPageRef.current);
      }
    },
    [apply, persistMode],
  );

  const beginGesture = useCallback(
    (source: 'pdf' | 'md') => {
      gestureDriverRef.current = source;
      if (gestureTimerRef.current != null) {
        window.clearTimeout(gestureTimerRef.current);
      }
      // Safety: release if scrollend never fires.
      gestureTimerRef.current = window.setTimeout(() => {
        gestureDriverRef.current = 'none';
        gestureTimerRef.current = null;
      }, Math.max(settleMs * 4, 1000));
    },
    [settleMs],
  );

  const endGesture = useCallback(() => {
    gestureDriverRef.current = 'none';
    if (gestureTimerRef.current != null) {
      window.clearTimeout(gestureTimerRef.current);
      gestureTimerRef.current = null;
    }
  }, []);

  return {
    activePage,
    syncMode,
    followMarkdown: modeFollowsMarkdown(syncMode),
    followPdf: modeFollowsPdf(syncMode),
    syncEnabled: syncMode !== 'none',
    driver,
    setSyncMode,
    setPageFromPdf,
    setPageFromMd,
    setPageFromExternal,
    beginGesture,
    endGesture,
  };
}

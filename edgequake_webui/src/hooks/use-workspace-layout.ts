/**
 * SPEC-155 — Documents docking workspace layout state.
 */

"use client";

import { useMediaQuery } from "@/hooks/use-media-query";
import {
  applyPreset,
  defaultWorkspaceDocument,
  dockZone,
  MOBILE_BREAKPOINT_PX,
  mobileStackTree,
  readWorkspaceLayout,
  setMaximized,
  setZoneCollapsed,
  WORKSPACE_PRESET_ORDER,
  writeWorkspaceLayout,
  type DockEdge,
  type WorkspaceLayoutDocument,
  type WorkspacePresetId,
  type WorkspaceZoneId,
} from "@/lib/documents/workspace-layout";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

export interface UseWorkspaceLayoutOptions {
  /** When true, auto-collapse Runs to a rail (no live work). */
  runsIdle?: boolean;
}

export interface UseWorkspaceLayoutReturn {
  doc: WorkspaceLayoutDocument;
  /** Tree used for rendering (mobile stack overrides on narrow viewports). */
  renderTree: WorkspaceLayoutDocument["tree"];
  isMobile: boolean;
  announcement: string;
  applyPresetId: (id: WorkspacePresetId) => void;
  reset: () => void;
  dock: (
    dragged: WorkspaceZoneId,
    target: WorkspaceZoneId,
    edge: DockEdge,
  ) => void;
  setCollapsed: (zone: WorkspaceZoneId, collapsed: boolean) => void;
  toggleCollapsed: (zone: WorkspaceZoneId) => void;
  setMaximizedZone: (zone: WorkspaceZoneId | null) => void;
  isCollapsed: (zone: WorkspaceZoneId) => boolean;
  clearAnnouncement: () => void;
}

export function useWorkspaceLayout(
  options: UseWorkspaceLayoutOptions = {},
): UseWorkspaceLayoutReturn {
  const { runsIdle = true } = options;
  const [doc, setDoc] = useState<WorkspaceLayoutDocument>(() =>
    defaultWorkspaceDocument(),
  );
  const [hydrated, setHydrated] = useState(false);
  const [announcement, setAnnouncement] = useState("");
  const isMobile = useMediaQuery(`(max-width: ${MOBILE_BREAKPOINT_PX - 1}px)`);
  /** User explicitly collapsed/expanded Runs — auto-rail must not override. */
  const runsUserOverride = useRef(false);
  const prevRunsIdle = useRef(runsIdle);
  const runsSyncedAfterHydrate = useRef(false);
  const runsIdleRef = useRef(runsIdle);
  runsIdleRef.current = runsIdle;

  const presetDoc = useCallback((id: WorkspacePresetId) => {
    return applyPreset(id, { runsIdle: runsIdleRef.current });
  }, []);

  useEffect(() => {
    setDoc(readWorkspaceLayout());
    setHydrated(true);
  }, []);

  useEffect(() => {
    if (!hydrated) return;
    writeWorkspaceLayout(doc);
  }, [doc, hydrated]);

  // Smart Runs rail: collapse when idle; expand when work appears (including
  // first paint after hydrate if localStorage left Runs collapsed).
  useEffect(() => {
    if (!hydrated || runsUserOverride.current) return;
    const wasIdle = prevRunsIdle.current;
    prevRunsIdle.current = runsIdle;
    const firstSync = !runsSyncedAfterHydrate.current;
    runsSyncedAfterHydrate.current = true;

    if (runsIdle) {
      if (!doc.collapsed.runs) {
        setDoc((d) => setZoneCollapsed(d, "runs", true));
      }
      return;
    }

    if (doc.collapsed.runs && (wasIdle || firstSync)) {
      setDoc((d) => setZoneCollapsed(d, "runs", false));
    }
  }, [runsIdle, hydrated, doc.collapsed.runs]);

  // Alt+1..4 presets
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!event.altKey || event.metaKey || event.ctrlKey) return;
      const target = event.target as HTMLElement | null;
      const tag = target?.tagName?.toLowerCase();
      if (
        tag === "input" ||
        tag === "textarea" ||
        tag === "select" ||
        target?.isContentEditable
      ) {
        return;
      }
      const idx = Number(event.key) - 1;
      if (idx >= 0 && idx < WORKSPACE_PRESET_ORDER.length) {
        event.preventDefault();
        const id = WORKSPACE_PRESET_ORDER[idx]!;
        setDoc(presetDoc(id));
        runsUserOverride.current = false;
        setAnnouncement(`Layout: ${id}`);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [presetDoc]);

  const applyPresetId = useCallback((id: WorkspacePresetId) => {
    runsUserOverride.current = false;
    const next = presetDoc(id);
    setDoc(next);
    setAnnouncement(`Layout: ${next.presetId}`);
  }, [presetDoc]);

  const reset = useCallback(() => {
    runsUserOverride.current = false;
    const next = presetDoc("classic");
    setDoc(next);
    setAnnouncement("Layout reset to Classic");
  }, [presetDoc]);

  const dock = useCallback(
    (dragged: WorkspaceZoneId, target: WorkspaceZoneId, edge: DockEdge) => {
      setDoc((d) => {
        const tree = dockZone(d.tree, dragged, target, edge);
        return {
          ...d,
          tree,
          presetId: null,
          maximized: null,
        };
      });
      setAnnouncement(
        edge === "center"
          ? `Swapped ${dragged} with ${target}`
          : `Docked ${dragged} to the ${edge} of ${target}`,
      );
    },
    [],
  );

  const setCollapsed = useCallback(
    (zone: WorkspaceZoneId, collapsed: boolean) => {
      if (zone === "runs") runsUserOverride.current = true;
      setDoc((d) => setZoneCollapsed(d, zone, collapsed));
    },
    [],
  );

  const toggleCollapsed = useCallback((zone: WorkspaceZoneId) => {
    if (zone === "runs") runsUserOverride.current = true;
    setDoc((d) => setZoneCollapsed(d, zone, !(d.collapsed[zone] ?? false)));
  }, []);

  const setMaximizedZone = useCallback((zone: WorkspaceZoneId | null) => {
    setDoc((d) => setMaximized(d, zone));
  }, []);

  const isCollapsed = useCallback(
    (zone: WorkspaceZoneId) => Boolean(doc.collapsed[zone]),
    [doc.collapsed],
  );

  const renderTree = useMemo(() => {
    if (isMobile) return mobileStackTree();
    if (doc.maximized) {
      // Maximized: still keep all zones in tree for identity, but UI will
      // hide non-maximized panels. Tree itself stays intact for restore.
      return doc.tree;
    }
    return doc.tree;
  }, [isMobile, doc.tree, doc.maximized]);

  const clearAnnouncement = useCallback(() => setAnnouncement(""), []);

  return {
    doc,
    renderTree,
    isMobile,
    announcement,
    applyPresetId,
    reset,
    dock,
    setCollapsed,
    toggleCollapsed,
    setMaximizedZone,
    isCollapsed,
    clearAnnouncement,
  };
}

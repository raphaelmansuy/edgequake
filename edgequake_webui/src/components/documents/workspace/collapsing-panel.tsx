"use client";

import { ZONE_RAIL_PX } from "@/lib/documents/workspace-layout";
import { panelSizePercent } from "@/lib/documents/panel-size-percent";
import { cn } from "@/lib/utils";
import { useEffect, type ReactNode } from "react";
import { Panel, usePanelRef } from "react-resizable-panels";

export interface CollapsingPanelProps {
  id: string;
  defaultSize: string | number;
  minSize: number;
  collapsedSize?: number;
  /** When true, imperatively collapse to collapsedSize (px). */
  collapsed: boolean;
  /**
   * When set (and not collapsed), imperatively resize to this percentage.
   * Needed because defaultSize is mount-only in react-resizable-panels.
   */
  sizePercent?: number;
  className?: string;
  children: ReactNode;
}

/**
 * Panel that syncs our zone-collapsed state to react-resizable-panels'
 * imperative collapse API — otherwise a railed zone keeps its % width
 * and paints a large empty void next to the rail.
 *
 * After expand(), also resize to defaultSize: panels that mounted while
 * collapsed often keep a ~3% sibling share and stay unusable.
 */
export function CollapsingPanel({
  id,
  defaultSize,
  minSize: _zoneMinPx,
  collapsedSize = ZONE_RAIL_PX,
  collapsed,
  sizePercent,
  className,
  children,
}: CollapsingPanelProps) {
  const panelRef = usePanelRef();
  const mountDefault = collapsed ? collapsedSize : defaultSize;

  useEffect(() => {
    const panel = panelRef.current;
    if (!panel) return;
    try {
      if (collapsed) {
        if (!panel.isCollapsed()) panel.collapse();
        return;
      }
      if (panel.isCollapsed()) {
        panel.expand();
      }
      const targetPct =
        typeof sizePercent === "number" && Number.isFinite(sizePercent)
          ? sizePercent
          : typeof defaultSize === "string" && !defaultSize.endsWith("px")
            ? Number(defaultSize.replace("%", ""))
            : typeof defaultSize === "number" && defaultSize <= 100
              ? defaultSize
              : null;
      if (targetPct != null && Number.isFinite(targetPct)) {
        const current = panelSizePercent(panel.getSize());
        if (current == null || Math.abs(current - targetPct) > 1.5) {
          panel.resize(`${targetPct}%`);
        }
      }
    } catch {
      // Group may have unmounted (mobile stack / HMR); next paint remounts.
    }
  }, [collapsed, sizePercent, defaultSize, panelRef]);

  return (
    <Panel
      id={id}
      panelRef={panelRef}
      defaultSize={mountDefault}
      // Do not pass ZONE_MIN as Panel minSize: react-resizable-panels
      // auto-collapses when computed size < minSize (library-center Intake
      // at ~16% was snapping to a 28px column while still painting dropzone).
      minSize={collapsed ? collapsedSize : 1}
      className={cn("flex h-full min-h-0 min-w-0 flex-col", className)}
      collapsible
      collapsedSize={collapsedSize}
    >
      {children}
    </Panel>
  );
}

export default CollapsingPanel;

"use client";

import {
  minSizeForSubtree,
  ZONE_MIN_PX,
  ZONE_RAIL_PX,
  type DockEdge,
  type LayoutNode,
  type WorkspaceZoneId,
} from "@/lib/documents/workspace-layout";
import { cn } from "@/lib/utils";
import {
  DndContext,
  DragOverlay,
  PointerSensor,
  KeyboardSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
  type DragStartEvent,
  closestCenter,
} from "@dnd-kit/core";
import type { ReactNode } from "react";
import { useCallback, useEffect, useState } from "react";
import { Group, Separator } from "react-resizable-panels";
import { CollapsingPanel } from "./collapsing-panel";
import { WorkspaceZone } from "./workspace-zone";

export type ZoneContentMap = Record<WorkspaceZoneId, ReactNode>;
export type ZoneBadgeMap = Partial<Record<WorkspaceZoneId, number | string | null>>;

export interface DocumentsWorkspaceProps {
  tree: LayoutNode;
  contents: ZoneContentMap;
  badges?: ZoneBadgeMap;
  collapsed: Partial<Record<WorkspaceZoneId, boolean>>;
  maximized: WorkspaceZoneId | null;
  isMobile?: boolean;
  announcement?: string;
  onClearAnnouncement?: () => void;
  onDock: (
    dragged: WorkspaceZoneId,
    target: WorkspaceZoneId,
    edge: DockEdge,
  ) => void;
  onToggleCollapse: (zone: WorkspaceZoneId) => void;
  onMaximize: (zone: WorkspaceZoneId | null) => void;
  className?: string;
}

function parseDropId(id: string | number): {
  zoneId: WorkspaceZoneId;
  edge: DockEdge;
} | null {
  const raw = String(id);
  const [zoneId, edge] = raw.split("::");
  if (
    (zoneId === "intake" || zoneId === "runs" || zoneId === "library") &&
    (edge === "left" ||
      edge === "right" ||
      edge === "top" ||
      edge === "bottom" ||
      edge === "center")
  ) {
    return { zoneId, edge };
  }
  return null;
}


function leafZones(node: LayoutNode): WorkspaceZoneId[] {
  if (node.type === "leaf") return [node.zone];
  return [...leafZones(node.children[0]), ...leafZones(node.children[1])];
}

export function DocumentsWorkspace({
  tree,
  contents,
  badges,
  collapsed,
  maximized,
  isMobile = false,
  announcement = "",
  onClearAnnouncement,
  onDock,
  onToggleCollapse,
  onMaximize,
  className,
}: DocumentsWorkspaceProps) {
  const [dragging, setDragging] = useState<WorkspaceZoneId | null>(null);

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
    useSensor(KeyboardSensor),
  );

  useEffect(() => {
    if (!announcement || !onClearAnnouncement) return;
    const t = window.setTimeout(onClearAnnouncement, 2500);
    return () => window.clearTimeout(t);
  }, [announcement, onClearAnnouncement]);

  const onDragStart = useCallback((event: DragStartEvent) => {
    const id = String(event.active.id);
    if (id === "intake" || id === "runs" || id === "library") {
      setDragging(id);
    }
  }, []);

  const onDragEnd = useCallback(
    (event: DragEndEvent) => {
      setDragging(null);
      const activeId = String(event.active.id);
      if (
        activeId !== "intake" &&
        activeId !== "runs" &&
        activeId !== "library"
      ) {
        return;
      }
      if (!event.over) return;
      const parsed = parseDropId(event.over.id);
      if (!parsed) return;
      if (parsed.zoneId === activeId) return;
      onDock(activeId, parsed.zoneId, parsed.edge);
    },
    [onDock],
  );

  const onDragCancel = useCallback(() => setDragging(null), []);

  const renderZone = useCallback(
    (
      zone: WorkspaceZoneId,
      railOrientation: "vertical" | "horizontal" = "vertical",
    ) => {
      if (maximized && maximized !== zone) {
        return null;
      }
      return (
        <WorkspaceZone
          key={zone}
          zone={zone}
          collapsed={!maximized && Boolean(collapsed[zone])}
          maximized={maximized === zone}
          showDockTargets={dragging != null && dragging !== zone}
          badge={badges?.[zone]}
          disableDrag={isMobile || maximized != null}
          railOrientation={railOrientation}
          onToggleCollapse={() => onToggleCollapse(zone)}
          onMaximize={() => onMaximize(zone)}
          onRestore={() => onMaximize(null)}
          onDock={(target, edge) => onDock(zone, target, edge)}
        >
          {contents[zone]}
        </WorkspaceZone>
      );
    },
    [
      badges,
      collapsed,
      contents,
      dragging,
      isMobile,
      maximized,
      onDock,
      onMaximize,
      onToggleCollapse,
    ],
  );

  const renderNode = useCallback(
    (node: LayoutNode, path: string): ReactNode => {
      if (maximized) {
        return renderZone(maximized);
      }
      if (node.type === "leaf") {
        return renderZone(node.zone);
      }

      if (isMobile) {
        // Stack without splitters on narrow viewports.
        return (
          <div
            className="flex min-h-0 min-w-0 flex-1 flex-col gap-0"
            data-testid={`workspace-mobile-stack-${path}`}
          >
            {leafZones(node).map((z) => {
              const railed = Boolean(collapsed[z]) && !maximized;
              return (
                <div
                  key={z}
                  className={cn(
                    "min-h-0 border-b last:border-b-0",
                    railed
                      ? "h-7 shrink-0"
                      : z === "library"
                        ? "flex-1"
                        : "shrink-0",
                  )}
                  style={
                    railed
                      ? { height: ZONE_RAIL_PX }
                      : z === "intake"
                        ? { minHeight: ZONE_MIN_PX.intake.height }
                        : undefined
                  }
                  data-mobile-zone={z}
                  data-railed={railed ? "true" : "false"}
                >
                  {renderZone(z, "horizontal")}
                </div>
              );
            })}
          </div>
        );
      }

      const orientation = node.orientation;
      const [left, right] = node.children;
      const leftZones = leafZones(left);
      const rightZones = leafZones(right);
      // If every zone under a child is collapsed to rail, give it a tiny default.
      const leftAllCollapsed = leftZones.every((z) => collapsed[z]);
      const rightAllCollapsed = rightZones.every((z) => collapsed[z]);

      // Collapsed rails claim almost no space (thin bar / strip).
      // toolsSlim: classic top band — keep short when Runs is railed.
      // toolsNarrow: side dock — keep Upload column slim so Library can breathe.
      const toolsSlim =
        orientation === "vertical" &&
        leftZones.includes("intake") &&
        leftZones.includes("runs") &&
        Boolean(collapsed.runs) &&
        !collapsed.intake;
      const toolsOnRight =
        orientation === "horizontal" &&
        leftZones.includes("library") &&
        !leftZones.includes("intake") &&
        rightZones.includes("intake");
      const toolsOnLeft =
        orientation === "horizontal" &&
        rightZones.includes("library") &&
        !rightZones.includes("intake") &&
        leftZones.includes("intake");
      const toolsNarrow =
        (toolsOnLeft || toolsOnRight) &&
        Boolean(collapsed.runs) &&
        !collapsed.intake;

      const leftDefault = leftAllCollapsed
        ? 3
        : rightAllCollapsed
          ? 97
          : node.sizes[0];
      const rightDefault = 100 - leftDefault;

      const leftMin = leftAllCollapsed
        ? ZONE_RAIL_PX
        : minSizeForSubtree(left, orientation, collapsed);
      const rightMin = rightAllCollapsed
        ? ZONE_RAIL_PX
        : minSizeForSubtree(right, orientation, collapsed);

      const viewport =
        typeof window !== "undefined" ? window.innerWidth : 1280;
      const viewportH =
        typeof window !== "undefined" ? window.innerHeight : 800;

      const slimPercent = toolsSlim
        ? Math.max(
            10,
            Math.round((ZONE_MIN_PX.intake.height / Math.max(viewportH, 1)) * 100) +
              2,
          )
        : null;
      // Cap tools dock ~280px so Library owns horizontal space when browsing.
      const narrowToolsPercent = toolsNarrow
        ? Math.min(
            28,
            Math.max(
              18,
              Math.round(
                ((ZONE_MIN_PX.intake.width + 48) / Math.max(viewport, 1)) * 100,
              ) + 1,
            ),
          )
        : null;

      let leftDefaultResolved = leftDefault;
      let rightDefaultResolved = rightDefault;
      if (toolsSlim && slimPercent != null) {
        leftDefaultResolved = slimPercent;
        rightDefaultResolved = 100 - slimPercent;
      } else if (toolsNarrow && narrowToolsPercent != null) {
        if (toolsOnRight) {
          leftDefaultResolved = 100 - narrowToolsPercent;
          rightDefaultResolved = narrowToolsPercent;
        } else {
          leftDefaultResolved = narrowToolsPercent;
          rightDefaultResolved = 100 - narrowToolsPercent;
        }
      }

      // Drive imperative resize for slim/narrow tools bands; CollapsingPanel
      // also restores defaultSize after expand so rail siblings recover share.
      const forceSizePercent = toolsSlim || toolsNarrow;

      const childRail =
        orientation === "vertical" ? "horizontal" : "vertical";

      const renderChild = (child: typeof left, childPath: string) => {
        if (child.type === "leaf") {
          return renderZone(child.zone, childRail);
        }
        return renderNode(child, childPath);
      };

      return (
        <Group
          key={path}
          id={`workspace-split-${path}`}
          orientation={orientation}
          className="min-h-0 min-w-0 flex-1"
          data-testid={`workspace-split-${path}`}
          resizeTargetMinimumSize={{ coarse: 24, fine: 8 }}
        >
          <CollapsingPanel
            id={`${path}-0`}
            defaultSize={leftAllCollapsed ? ZONE_RAIL_PX : `${leftDefaultResolved}`}
            minSize={leftMin}
            collapsed={leftAllCollapsed}
            collapsedSize={ZONE_RAIL_PX}
            sizePercent={
              leftAllCollapsed
                ? undefined
                : forceSizePercent
                  ? leftDefaultResolved
                  : undefined
            }
            className="min-h-0 min-w-0"
          >
            {renderChild(left, `${path}0`)}
          </CollapsingPanel>
          <Separator
            className={cn(
              "relative z-10 bg-transparent transition-colors",
              "after:absolute after:bg-border/60 after:content-['']",
              "hover:after:bg-sky-500/80 data-[separator=active]:after:bg-sky-500",
              orientation === "horizontal"
                ? "w-2 cursor-col-resize after:inset-y-0 after:left-1/2 after:w-px after:-translate-x-1/2"
                : "h-2 cursor-row-resize after:inset-x-0 after:top-1/2 after:h-px after:-translate-y-1/2",
            )}
            data-testid={`workspace-splitter-${path}`}
          />
          <CollapsingPanel
            id={`${path}-1`}
            defaultSize={
              rightAllCollapsed ? ZONE_RAIL_PX : `${rightDefaultResolved}`
            }
            minSize={rightMin}
            collapsed={rightAllCollapsed}
            collapsedSize={ZONE_RAIL_PX}
            sizePercent={
              rightAllCollapsed
                ? undefined
                : forceSizePercent
                  ? rightDefaultResolved
                  : undefined
            }
            className="min-h-0 min-w-0"
          >
            {renderChild(right, `${path}1`)}
          </CollapsingPanel>
        </Group>
      );
    },
    [collapsed, isMobile, maximized, renderZone],
  );

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      onDragStart={onDragStart}
      onDragEnd={onDragEnd}
      onDragCancel={onDragCancel}
    >
      <div
        className={cn(
          "flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden",
          className,
        )}
        data-testid="documents-workspace"
        data-mobile={isMobile ? "true" : "false"}
        data-maximized={maximized ?? ""}
      >
        <div aria-live="polite" className="sr-only" data-testid="workspace-announcement">
          {announcement}
        </div>
        {renderNode(tree, "r")}
      </div>
      <DragOverlay dropAnimation={null}>
        {dragging ? (
          <div className="rounded-md border bg-background px-3 py-1.5 text-xs font-medium shadow-lg">
            {dragging}
          </div>
        ) : null}
      </DragOverlay>
    </DndContext>
  );
}

export default DocumentsWorkspace;

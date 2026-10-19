"use client";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  edgeLabel,
  WORKSPACE_ZONES,
  zoneLabel,
  type DockEdge,
  type WorkspaceZoneId,
} from "@/lib/documents/workspace-layout";
import { cn } from "@/lib/utils";
import { useDraggable } from "@dnd-kit/core";
import { CSS } from "@dnd-kit/utilities";
import {
  ChevronDown,
  ChevronRight,
  GripVertical,
  Maximize2,
  Minimize2,
  MoreHorizontal,
} from "lucide-react";
import type { ReactNode } from "react";
import { DockOverlay } from "./dock-overlay";
import { ZoneRail } from "./zone-rail";

export interface WorkspaceZoneProps {
  zone: WorkspaceZoneId;
  children: ReactNode;
  collapsed?: boolean;
  maximized?: boolean;
  showDockTargets?: boolean;
  badge?: number | string | null;
  disableDrag?: boolean;
  /** Side strip vs top/bottom bar when collapsed. */
  railOrientation?: "vertical" | "horizontal";
  onToggleCollapse: () => void;
  onMaximize: () => void;
  onRestore: () => void;
  onDock: (target: WorkspaceZoneId, edge: DockEdge) => void;
  className?: string;
}

export function WorkspaceZone({
  zone,
  children,
  collapsed = false,
  maximized = false,
  showDockTargets = false,
  badge,
  disableDrag = false,
  railOrientation = "vertical",
  onToggleCollapse,
  onMaximize,
  onRestore,
  onDock,
  className,
}: WorkspaceZoneProps) {
  const { attributes, listeners, setNodeRef, transform, isDragging } =
    useDraggable({
      id: zone,
      data: { zone },
      disabled: disableDrag || collapsed,
    });

  const style = transform
    ? { transform: CSS.Translate.toString(transform) }
    : undefined;

  if (collapsed && !maximized) {
    return (
      <ZoneRail
        zone={zone}
        badge={badge}
        onExpand={onToggleCollapse}
        orientation={railOrientation}
      />
    );
  }

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={cn(
        "relative flex h-full min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-background",
        isDragging && "opacity-50",
        className,
      )}
      data-testid={`workspace-zone-${zone}`}
      data-zone={zone}
      data-collapsed="false"
      data-maximized={maximized ? "true" : "false"}
    >
      <div
        className="group/title flex h-7 shrink-0 items-center gap-1 border-b border-border/60 bg-background px-2"
        data-testid={`workspace-zone-title-${zone}`}
      >
        <button
          type="button"
          className={cn(
            "flex h-6 w-4 shrink-0 cursor-grab items-center justify-center rounded-sm",
            "text-muted-foreground/50 opacity-0 hover:bg-muted hover:text-foreground",
            "group-hover/title:opacity-100 group-focus-within/title:opacity-100 focus-visible:opacity-100",
            "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
            "active:cursor-grabbing",
            disableDrag && "pointer-events-none opacity-30",
          )}
          aria-label={`Drag ${zoneLabel(zone)}`}
          data-testid={`workspace-zone-handle-${zone}`}
          {...listeners}
          {...attributes}
        >
          <GripVertical className="h-3 w-3" aria-hidden="true" />
        </button>

        <button
          type="button"
          className="flex min-w-0 flex-1 items-center gap-1.5 rounded-sm px-1 py-1 text-left text-xs font-semibold tracking-tight text-foreground/80 hover:text-foreground"
          onClick={onToggleCollapse}
          aria-expanded={!collapsed}
          data-testid={`workspace-zone-toggle-${zone}`}
        >
          {collapsed ? (
            <ChevronRight className="h-3 w-3 shrink-0" aria-hidden="true" />
          ) : (
            <ChevronDown className="h-3 w-3 shrink-0" aria-hidden="true" />
          )}
          <span className="truncate">{zoneLabel(zone)}</span>
          {badge != null && badge !== 0 && badge !== "" ? (
            <span className="ml-1 rounded-full bg-muted px-1.5 text-[10px] font-medium tabular-nums text-muted-foreground">
              {badge}
            </span>
          ) : null}
        </button>

        {zone !== "intake" || maximized ? (
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="h-6 w-6 text-muted-foreground opacity-0 group-hover/title:opacity-100 group-focus-within/title:opacity-100 focus-visible:opacity-100"
            onClick={maximized ? onRestore : onMaximize}
            aria-label={maximized ? "Restore layout" : `Maximize ${zoneLabel(zone)}`}
            data-testid={`workspace-zone-maximize-${zone}`}
          >
            {maximized ? (
              <Minimize2 className="h-3 w-3" />
            ) : (
              <Maximize2 className="h-3 w-3" />
            )}
          </Button>
        ) : null}

        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="h-6 w-6 text-muted-foreground opacity-0 group-hover/title:opacity-100 group-focus-within/title:opacity-100 focus-visible:opacity-100 data-[state=open]:opacity-100"
              aria-label={`${zoneLabel(zone)} menu`}
              data-testid={`workspace-zone-menu-${zone}`}
            >
              <MoreHorizontal className="h-3.5 w-3.5" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-48">
            <DropdownMenuLabel>Move to</DropdownMenuLabel>
            {WORKSPACE_ZONES.filter((z) => z !== zone).map((target) => (
              <DropdownMenuSub key={target}>
                <DropdownMenuSubTrigger>
                  {zoneLabel(target)}
                </DropdownMenuSubTrigger>
                <DropdownMenuSubContent>
                  {(
                    ["left", "right", "top", "bottom", "center"] as DockEdge[]
                  ).map((edge) => (
                    <DropdownMenuItem
                      key={edge}
                      data-testid={`workspace-move-${zone}-to-${target}-${edge}`}
                      onSelect={() => onDock(target, edge)}
                    >
                      {edgeLabel(edge)}
                    </DropdownMenuItem>
                  ))}
                </DropdownMenuSubContent>
              </DropdownMenuSub>
            ))}
            <DropdownMenuSeparator />
            <DropdownMenuItem onSelect={onToggleCollapse}>
              Collapse to rail
            </DropdownMenuItem>
            <DropdownMenuItem onSelect={maximized ? onRestore : onMaximize}>
              {maximized ? "Restore" : "Maximize"}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>

      <div
        className={cn(
          "relative flex min-h-0 min-w-0 flex-1 flex-col overscroll-contain",
          zone === "intake" ? "overflow-hidden" : "overflow-auto",
        )}
      >
        <div
          className={cn(
            "flex min-h-0 min-w-0 flex-1 flex-col",
            zone === "intake" && "min-h-0",
          )}
        >
          {children}
        </div>
        <DockOverlay zoneId={zone} active={showDockTargets} />
      </div>
    </div>
  );
}

export default WorkspaceZone;

"use client";

import { cn } from "@/lib/utils";
import type { WorkspaceZoneId } from "@/lib/documents/workspace-layout";
import { zoneLabel } from "@/lib/documents/workspace-layout";
import { FileText, PlayCircle, Upload } from "lucide-react";

const ZONE_ICON: Record<WorkspaceZoneId, typeof Upload> = {
  intake: Upload,
  runs: PlayCircle,
  library: FileText,
};

export interface ZoneRailProps {
  zone: WorkspaceZoneId;
  badge?: number | string | null;
  onExpand: () => void;
  /** Vertical strip (side) or horizontal bar (top/bottom). */
  orientation?: "vertical" | "horizontal";
  className?: string;
}

export function ZoneRail({
  zone,
  badge,
  onExpand,
  orientation = "vertical",
  className,
}: ZoneRailProps) {
  const Icon = ZONE_ICON[zone];
  const horizontal = orientation === "horizontal";

  return (
    <button
      type="button"
      className={cn(
        "flex shrink-0 items-center gap-1.5 text-muted-foreground",
        "bg-muted/30 hover:bg-muted/55 hover:text-foreground",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
        "transition-colors",
        // Fill the collapsed panel so no empty white void remains beside the label.
        horizontal
          ? "h-full min-h-7 w-full flex-row justify-center px-2"
          : "h-full w-full min-w-7 flex-col justify-center",
        className,
      )}
      onClick={onExpand}
      aria-label={`Expand ${zoneLabel(zone)}`}
      data-testid={`workspace-zone-rail-${zone}`}
      data-orientation={orientation}
    >
      <Icon className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
      <span
        className="text-[10px] font-medium uppercase tracking-wide"
        style={
          horizontal
            ? undefined
            : { writingMode: "vertical-rl", transform: "rotate(180deg)" }
        }
      >
        {zoneLabel(zone)}
      </span>
      {badge != null && badge !== 0 && badge !== "" ? (
        <span className="rounded-full bg-muted px-1.5 text-[10px] font-medium tabular-nums text-muted-foreground">
          {badge}
        </span>
      ) : null}
    </button>
  );
}

export default ZoneRail;

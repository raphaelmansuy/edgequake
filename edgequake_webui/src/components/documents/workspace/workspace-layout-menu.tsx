"use client";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  WORKSPACE_PRESET_ORDER,
  WORKSPACE_PRESETS,
  type WorkspacePresetId,
} from "@/lib/documents/workspace-layout";
import { Columns2, LayoutDashboard, PanelLeft, PanelRight, RotateCcw } from "lucide-react";
import { useTranslation } from "react-i18next";

const PRESET_ICONS: Record<WorkspacePresetId, typeof LayoutDashboard> = {
  classic: LayoutDashboard,
  "library-left": PanelLeft,
  "library-right": PanelRight,
  "library-center": Columns2,
};

export interface WorkspaceLayoutMenuProps {
  presetId: WorkspacePresetId | null;
  onApplyPreset: (id: WorkspacePresetId) => void;
  onReset: () => void;
}

export function WorkspaceLayoutMenu({
  presetId,
  onApplyPreset,
  onReset,
}: WorkspaceLayoutMenuProps) {
  const { t } = useTranslation();

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="h-8 w-8"
          title={t("documents.layout.menu", "Layout")}
          data-testid="workspace-layout-menu"
          aria-label={t("documents.layout.menu", "Layout")}
        >
          <LayoutDashboard className="h-4 w-4" aria-hidden="true" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-56">
        <DropdownMenuLabel>
          {t("documents.layout.presets", "Presets")}
        </DropdownMenuLabel>
        {WORKSPACE_PRESET_ORDER.map((id, index) => {
          const preset = WORKSPACE_PRESETS[id];
          const Icon = PRESET_ICONS[id];
          return (
            <DropdownMenuItem
              key={id}
              data-testid={`workspace-layout-preset-${id}`}
              data-active={presetId === id ? "true" : "false"}
              onSelect={() => onApplyPreset(id)}
              className="gap-2"
            >
              <Icon className="h-3.5 w-3.5" aria-hidden="true" />
              <span className="flex-1">{preset.label}</span>
              <kbd className="text-[10px] text-muted-foreground">Alt+{index + 1}</kbd>
            </DropdownMenuItem>
          );
        })}
        <DropdownMenuSeparator />
        <DropdownMenuItem
          data-testid="workspace-layout-reset"
          onSelect={() => onReset()}
          className="gap-2"
        >
          <RotateCcw className="h-3.5 w-3.5" aria-hidden="true" />
          {t("documents.layout.reset", "Reset layout")}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

export default WorkspaceLayoutMenu;

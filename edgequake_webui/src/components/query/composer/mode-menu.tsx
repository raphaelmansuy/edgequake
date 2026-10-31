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
import { useWorkspaceCapabilities } from "@/hooks/use-workspace-capabilities";
import { modesForProfile } from "@/lib/query/query-profile";
import { QUERY_MODE_META } from "@/lib/query/query-mode-meta";
import { cn } from "@/lib/utils";
import type { QueryMode } from "@/types";
import { Check, ChevronDown } from "lucide-react";
import { useTranslation } from "react-i18next";

interface ModeMenuProps {
  value: QueryMode;
  onChange: (mode: QueryMode) => void;
  disabled?: boolean;
  /** Compact chip for composer toolbar */
  compact?: boolean;
}

/** Outcome-worded mode menu (Smart default). Replaces header segmented control. */
export function ModeMenu({
  value,
  onChange,
  disabled,
  compact = true,
}: ModeMenuProps) {
  const { t } = useTranslation();
  // SPEC-180: a workspace without a graph lists only the modes it can run.
  const modes = modesForProfile(useWorkspaceCapabilities());
  const current = modes.find((m) => m.id === value) ?? modes[0] ?? QUERY_MODE_META[3]!;
  const Icon = current.icon;
  const label = t(`query.modes.${current.id}`, current.label);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          disabled={disabled}
          className={cn(
            "h-8 gap-1.5 px-2 text-xs font-medium text-muted-foreground hover:text-foreground",
            compact && "max-w-[9rem]",
          )}
          data-testid="query-mode-selector"
          data-tour="query-mode"
          aria-label={t("query.modes.groupLabel", "Query retrieval mode")}
        >
          <Icon className={cn("h-3.5 w-3.5 shrink-0", current.color)} />
          <span className="truncate">{label}</span>
          <ChevronDown className="h-3 w-3 shrink-0 text-muted-foreground" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="w-72">
        <DropdownMenuLabel>
          {t("query.modes.menuTitle", "Retrieval mode")}
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        {modes.map((mode) => {
          const ModeIcon = mode.icon;
          const modeLabel = t(`query.modes.${mode.id}`, mode.label);
          const description = t(
            `query.modes.${mode.id}Description`,
            mode.description,
          );
          const selected = value === mode.id;
          return (
            <DropdownMenuItem
              key={mode.id}
              data-testid={`query-mode-${mode.id}`}
              data-mode={mode.id}
              className="flex items-start gap-2 py-2"
              onSelect={() => onChange(mode.id)}
            >
              <ModeIcon
                className={cn("h-4 w-4 mt-0.5 shrink-0", mode.color)}
              />
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-1.5">
                  <span className="text-sm font-medium">{modeLabel}</span>
                  {mode.recommended ? (
                    <span className="text-[10px] uppercase tracking-wide text-primary">
                      {t("query.modes.recommendedBadge", "Recommended")}
                    </span>
                  ) : null}
                </div>
                <p className="text-xs text-muted-foreground line-clamp-2">
                  {description}
                </p>
                <p className="text-xs text-muted-foreground/80 mt-0.5">
                  {t("query.modes.apiName", "API mode")}: {mode.apiName}
                </p>
              </div>
              {selected ? (
                <Check className="h-4 w-4 shrink-0 text-primary" />
              ) : null}
            </DropdownMenuItem>
          );
        })}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

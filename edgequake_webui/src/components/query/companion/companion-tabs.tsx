/** SPEC-157 — Source | Graph segmented switch (one pane at a time). */
"use client";

import { cn } from "@/lib/utils";
import type { CompanionKind } from "@/lib/query/companion-pane";
import { FileText, Network } from "lucide-react";
import { useTranslation } from "react-i18next";

type PaneKind = Exclude<CompanionKind, "none">;

interface CompanionTabsProps {
  active: PaneKind;
  /** Panes with something to show (the active one is always available). */
  available: Record<PaneKind, boolean>;
  onSelect: (kind: PaneKind) => void;
  idPrefix: string;
  /** SPEC-180: false on a workspace without a graph; the tab is not drawn. */
  showGraph?: boolean;
}

export function companionTabId(prefix: string, kind: PaneKind) {
  return `${prefix}-tab-${kind}`;
}
export function companionPanelId(prefix: string) {
  return `${prefix}-panel`;
}

export function CompanionTabs({
  active,
  available,
  onSelect,
  idPrefix,
  showGraph = true,
}: CompanionTabsProps) {
  const { t } = useTranslation();
  const tabs: { kind: PaneKind; label: string; Icon: typeof FileText }[] = [
    { kind: "pdf", label: t("query.companion.source", "Source"), Icon: FileText },
    { kind: "graph", label: t("query.companion.graph", "Graph"), Icon: Network },
  ].filter((tab) => showGraph || tab.kind !== "graph") as {
    kind: PaneKind;
    label: string;
    Icon: typeof FileText;
  }[];

  return (
    <div
      role="tablist"
      aria-label={t("query.companion.title", "Companion pane")}
      className="flex rounded-lg bg-muted p-0.5"
    >
      {tabs.map(({ kind, label, Icon }) => {
        const selected = active === kind;
        return (
          <button
            key={kind}
            id={companionTabId(idPrefix, kind)}
            role="tab"
            type="button"
            aria-selected={selected}
            aria-controls={companionPanelId(idPrefix)}
            tabIndex={selected ? 0 : -1}
            disabled={!selected && !available[kind]}
            onClick={() => onSelect(kind)}
            onKeyDown={(e) => {
              if (e.key !== "ArrowLeft" && e.key !== "ArrowRight") return;
              const other = kind === "pdf" ? "graph" : "pdf";
              if (available[other]) {
                e.preventDefault();
                onSelect(other);
                requestAnimationFrame(() =>
                  document.getElementById(companionTabId(idPrefix, other))?.focus(),
                );
              }
            }}
            className={cn(
              "flex items-center gap-1.5 rounded-md px-2.5 py-1 text-xs font-medium transition-colors",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50",
              selected
                ? "bg-background text-foreground shadow-sm"
                : "text-muted-foreground hover:text-foreground disabled:opacity-40 disabled:hover:text-muted-foreground",
            )}
            data-testid={`companion-tab-${kind}`}
          >
            <Icon className="h-3.5 w-3.5" aria-hidden />
            {label}
          </button>
        );
      })}
    </div>
  );
}

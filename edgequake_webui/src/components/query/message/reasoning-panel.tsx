"use client";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { Brain, ChevronDown, ChevronRight, Clock } from "lucide-react";
import { memo, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface ReasoningPanelProps {
  thinking: string[];
  thinkingTimeMs?: number;
  /** Live streaming of an open <think> block */
  isLive?: boolean;
  isExpanded: boolean;
  onToggle: () => void;
}

export const ReasoningPanel = memo(function ReasoningPanel({
  thinking,
  thinkingTimeMs,
  isLive = false,
  isExpanded,
  onToggle,
}: ReasoningPanelProps) {
  const { t } = useTranslation();
  const [liveElapsedMs, setLiveElapsedMs] = useState(0);

  useEffect(() => {
    if (!isLive) return;
    const started = Date.now() - (thinkingTimeMs ?? 0);
    const tick = () => setLiveElapsedMs(Date.now() - started);
    tick();
    const id = window.setInterval(tick, 200);
    return () => window.clearInterval(id);
  }, [isLive, thinkingTimeMs]);

  if (thinking.length === 0) return null;

  const displayMs = isLive
    ? liveElapsedMs || thinkingTimeMs || 0
    : thinkingTimeMs;

  const label = isLive
    ? t("query.thinkingLive", "Thinking…")
    : thinkingTimeMs
      ? t("query.thoughtFor", "Thought for {{seconds}}s", {
          seconds: (thinkingTimeMs / 1000).toFixed(1),
        })
      : t("query.reasoning", "Reasoning");

  return (
    <div
      className="relative rounded-xl border bg-muted/20"
      data-testid="query-reasoning-panel"
      data-live={isLive ? "true" : "false"}
    >
      <Button
        type="button"
        variant="ghost"
        onClick={onToggle}
        className="flex items-center gap-2 w-full h-auto px-4 py-3 justify-start rounded-none"
        aria-expanded={isExpanded}
        aria-label={t("query.toggleReasoning", "Toggle reasoning details")}
      >
        {isExpanded ? (
          <ChevronDown className="h-4 w-4 text-muted-foreground" />
        ) : (
          <ChevronRight className="h-4 w-4 text-muted-foreground" />
        )}
        <Brain
          className={cn(
            "h-4 w-4 text-primary/70",
            isLive && "motion-safe:animate-pulse",
          )}
          aria-hidden
        />
        <span
          className={cn(
            "text-sm font-medium text-foreground/80",
            isLive && "motion-safe:animate-pulse",
          )}
        >
          {label}
        </span>
        {displayMs ? (
          <span className="text-xs text-muted-foreground ml-auto flex items-center gap-1 tabular-nums">
            <Clock className="h-3 w-3" />
            {(displayMs / 1000).toFixed(1)}s
          </span>
        ) : null}
      </Button>
      {isExpanded ? (
        <div className="absolute inset-x-0 top-full z-20 mt-1 rounded-xl border bg-background px-4 pb-4 shadow-md">
          <div
            className={cn(
              "text-sm text-muted-foreground whitespace-pre-wrap",
              "pl-4 pt-3 border-l-2 border-primary/30",
              "max-h-64 overflow-y-auto",
            )}
            data-testid="query-reasoning-body"
          >
            {thinking.join("\n\n")}
            {isLive ? (
              <span className="inline-block w-1.5 h-3 ml-0.5 bg-primary/60 align-middle motion-safe:animate-pulse" />
            ) : null}
          </div>
        </div>
      ) : null}
    </div>
  );
});

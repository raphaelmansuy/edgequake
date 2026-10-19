"use client";

/**
 * Waiting state shown between "question sent" and the first answer token.
 *
 * Motion is purposeful, not decorative: the stage label shimmers, three dots
 * wave, the four-step rail shows how far the pipeline got, an elapsed counter
 * and a gentle hint appear when the wait is long, and skeleton lines hold the
 * answer's space so text landing does not shift the page.
 */
import { useElapsedSeconds } from "@/hooks/use-elapsed-seconds";
import {
  ELAPSED_VISIBLE_AFTER_S,
  PENDING_STEPS,
  stepStates,
  waitHint,
  type PendingStep,
  type StepState,
} from "@/lib/query/pending-answer";
import type { StreamStage } from "@/lib/query/stream-session-reducer";
import { cn } from "@/lib/utils";
import { Check } from "lucide-react";
import { useTranslation } from "react-i18next";

interface StageTimelineProps {
  stage: StreamStage | null;
  detail?: string;
  sourceCount?: number;
  /** When the question was sent (ms epoch) — drives the elapsed counter. */
  startedAt?: number;
}

const SKELETON_WIDTHS = ["w-[92%]", "w-[78%]", "w-[54%]"] as const;

function WaveDots() {
  return (
    <span className="flex h-4 items-center gap-[3px]" aria-hidden="true">
      {[0, 1, 2].map((i) => (
        <span key={i} className="eq-wave-dot block h-1 w-1 rounded-full bg-primary" />
      ))}
    </span>
  );
}

function StepRail({ stage }: { stage: StreamStage | null }) {
  const { t } = useTranslation();
  const states = stepStates(stage);
  const labels: Record<PendingStep, string> = {
    retrieving: t("query.stage.stepSearch", "Search"),
    reading: t("query.stage.stepRead", "Read"),
    thinking: t("query.stage.stepThink", "Think"),
    generating: t("query.stage.stepWrite", "Write"),
  };
  return (
    <ol className="flex items-start gap-1.5" aria-hidden="true" data-testid="query-stage-steps">
      {PENDING_STEPS.map((step, i) => (
        <li key={step} className="flex min-w-0 flex-1 flex-col gap-1">
          <span
            className={cn(
              "h-[3px] w-full rounded-full transition-colors duration-500",
              railTone(states[i]),
            )}
          />
          <span
            className={cn(
              "flex items-center gap-1 truncate text-[10.5px] transition-colors duration-300",
              states[i] === "todo" ? "text-muted-foreground/70" : "text-foreground/80",
              states[i] === "active" && "font-medium",
            )}
          >
            {states[i] === "done" ? <Check className="h-2.5 w-2.5 text-primary" /> : null}
            {labels[step]}
          </span>
        </li>
      ))}
    </ol>
  );
}

function railTone(state: StepState): string {
  if (state === "done") return "bg-primary";
  if (state === "active") return "eq-step-active bg-primary/60";
  return "bg-border";
}

function SkeletonLines() {
  return (
    <div className="space-y-2 pt-1" aria-hidden="true" data-testid="query-answer-skeleton">
      {SKELETON_WIDTHS.map((w) => (
        <div key={w} className={cn("eq-skeleton-line h-3 rounded-full", w)} />
      ))}
    </div>
  );
}

export function StageTimeline({ stage, detail, sourceCount, startedAt }: StageTimelineProps) {
  const { t } = useTranslation();
  const active = stage !== "stopped" && stage !== "error" && stage !== "complete";
  const elapsed = useElapsedSeconds(startedAt, active);
  if (stage === "complete") return null;

  const label =
    stage === "reading"
      ? t("query.stage.reading", "Reading sources…")
      : stage === "thinking"
        ? t("query.stage.thinking", "Thinking…")
        : stage === "generating"
          ? t("query.stage.generating", "Writing answer…")
          : stage === "stopped"
            ? t("query.stage.stopped", "Stopped")
            : stage === "error"
              ? t("query.stage.error", "Failed")
              : t("query.stage.retrieving", "Searching knowledge…");

  const meta =
    typeof sourceCount === "number"
      ? t("query.stage.sourceCount", "{{count}} sources", { count: sourceCount })
      : detail;
  const hint = active ? waitHint(elapsed) : "none";

  return (
    <div className="mb-3 w-[26rem] max-w-full space-y-3" data-testid="query-stage-timeline" data-stage={stage ?? "retrieving"} role="status">
      <div className="flex items-center gap-2.5 text-sm">
        {active ? <WaveDots /> : null}
        <span className={cn("font-medium", active ? "eq-shimmer-text" : "text-foreground/80")}>{label}</span>
        {meta ? <span className="truncate text-xs text-muted-foreground">{meta}</span> : null}
        {active && elapsed >= ELAPSED_VISIBLE_AFTER_S ? (
          <span className="ml-auto text-xs tabular-nums text-muted-foreground" data-testid="query-stage-elapsed">
            {t("query.stage.elapsed", "{{seconds}}s", { seconds: elapsed })}
          </span>
        ) : null}
      </div>
      {active ? <StepRail stage={stage} /> : null}
      {hint !== "none" ? (
        <p className="text-xs text-muted-foreground motion-safe:animate-in motion-safe:fade-in" data-testid="query-stage-hint">
          {hint === "slow"
            ? t("query.stage.slowHint", "This is taking a while — larger local models can need a moment.")
            : t("query.stage.stillWorking", "Still working on it…")}
        </p>
      ) : null}
      {active ? <SkeletonLines /> : null}
    </div>
  );
}

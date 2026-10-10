"use client";

import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { useActiveConversationId } from "@/stores/use-query-ui-store";
import { useOpenAnswerGraph } from "@/hooks/use-open-answer-graph";
import { useOpenSource } from "@/hooks/use-open-source";
import { hasAnswerGraph } from "@/lib/query/answer-graph";
import { locationFromDocumentClick } from "@/lib/query/companion-pane";
import { cn } from "@/lib/utils";
import { Sparkles } from "lucide-react";
import { buildCitationResolver } from "@/lib/citations/resolve-citation";
import { memo, useCallback, useEffect, useMemo, useReducer, useState } from "react";
import { useRouter } from "next/navigation";
import { useTranslation } from "react-i18next";
import { StreamingMarkdownRenderer } from "../markdown";
import { SourceCitations } from "../source-citations";
import {
  parseCOTContent,
  parseCOTStreaming,
} from "@/lib/query/parse-cot-streaming";
import {
  createReasoningPanelState,
  isReasoningPanelExpanded,
  isReasoningPanelLive,
  reduceReasoningPanel,
  reasoningEventFromStream,
} from "@/lib/query/reasoning-panel-machine";
import { MessageActions } from "./message-actions";
import { MessageError } from "./message-error";
import { ReasoningPanel } from "./reasoning-panel";
import { SourceChips } from "./source-chips";
import { StageTimeline } from "./stage-timeline";
import type { ChatMessageProps } from "./types";
import type { StreamStage } from "@/lib/query/stream-session-reducer";
import { setMessageFeedback } from "@/lib/api/conversations";
import { toast } from "sonner";

function formatRelativeTime(ts: number, locale: string): string {
  const diffSec = Math.round((Date.now() - ts) / 1000);
  const rtf = new Intl.RelativeTimeFormat(locale, { numeric: "auto" });
  if (Math.abs(diffSec) < 60) return rtf.format(-diffSec, "second");
  const diffMin = Math.round(diffSec / 60);
  if (Math.abs(diffMin) < 60) return rtf.format(-diffMin, "minute");
  const diffHr = Math.round(diffMin / 60);
  if (Math.abs(diffHr) < 24) return rtf.format(-diffHr, "hour");
  return new Date(ts).toLocaleTimeString(locale, {
    hour: "2-digit",
    minute: "2-digit",
  });
}

export const AssistantMessage = memo(function AssistantMessage({
  message,
  isLast,
  onCopy,
  onRegenerate,
  onRetry,
  onContinue,
  onFeedback,
  showMetadata = true,
  stage,
  stageDetail,
}: ChatMessageProps) {
  const { t, i18n } = useTranslation();
  const router = useRouter();
  const openSource = useOpenSource();
  const openAnswerGraph = useOpenAnswerGraph();
  const sessionId = useActiveConversationId();
  const [copied, setCopied] = useState(false);
  const [reasoningUi, dispatchReasoning] = useReducer(
    reduceReasoningPanel,
    undefined,
    createReasoningPanelState,
  );
  // Counter (not boolean) so every "+N more" click re-opens a collapsed panel.
  const [sourcesOpenSignal, setSourcesOpenSignal] = useState(0);
  const [feedback, setFeedback] = useState<"up" | "down" | null>(
    message.feedbackRating ?? null,
  );
  const resolveCitation = useMemo(
    () => buildCitationResolver(message.context),
    [message.context],
  );

  const handleFeedback = useCallback(
    async (rating: "up" | "down" | null) => {
      setFeedback(rating);
      onFeedback?.(rating);
      if (!sessionId) return;
      try {
        await setMessageFeedback(sessionId, message.id, rating);
      } catch {
        toast.error(t("query.feedback.failed", "Could not save feedback"));
      }
    },
    [message.id, onFeedback, sessionId, t],
  );

  const handleCopy = useCallback(async () => {
    const parsed = parseCOTContent(message.content);
    const textToCopy = parsed.response || message.content;
    try {
      await navigator.clipboard.writeText(textToCopy);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
      onCopy?.();
    } catch (err) {
      console.error("Copy failed:", err);
    }
  }, [message.content, onCopy]);

  const hasGraphEntities = hasAnswerGraph(message.context);

  const handleShowOnGraph = useCallback(
    () => openAnswerGraph(message.id, message.context),
    [message.id, message.context, openAnswerGraph],
  );

  const parsedLive = parseCOTStreaming(message.content);
  const parsed = parseCOTContent(message.content);
  const thinkingBlocks =
    parsedLive.thinking.length > 0 ? parsedLive.thinking : parsed.thinking;
  const hasThinking = thinkingBlocks.some((t) => t.length > 0) || parsedLive.open;
  const displayContent = parsedLive.open
    ? parsedLive.response
    : parsed.response || parsedLive.response;
  const hasResponseText = Boolean(displayContent.trim());
  const isLiveThinking = Boolean(message.isStreaming && parsedLive.open);
  const showStage =
    message.isStreaming &&
    stage &&
    stage !== "complete" &&
    stage !== "thinking" &&
    !displayContent &&
    !isLiveThinking;

  // Drive panel via pure state machine (no boolean sync loops)
  useEffect(() => {
    dispatchReasoning(
      reasoningEventFromStream({
        isStreaming: Boolean(message.isStreaming),
        cotOpen: parsedLive.open,
        hasResponseText,
      }),
    );
  }, [message.isStreaming, parsedLive.open, hasResponseText]);

  const thinkingExpanded = isReasoningPanelExpanded(reasoningUi.phase);
  const panelLive = isReasoningPanelLive(reasoningUi.phase);

  if (message.isError) {
    return <MessageError message={message.content} onRetry={onRetry} />;
  }

  return (
    <div
      className="flex justify-start mb-6 group"
      role="article"
      aria-label={t("query.assistantMessage", "Assistant response")}
    >
      <div className="flex items-start gap-3 max-w-full min-w-0">
        <Avatar className="h-8 w-8 shrink-0 mt-1 ring-2 ring-primary/20">
          <AvatarFallback className="bg-primary text-primary-foreground">
            <Sparkles className="h-4 w-4" aria-hidden="true" />
          </AvatarFallback>
        </Avatar>

        <div className="space-y-2 min-w-0 flex-1">
          <div className="flex items-center gap-2 text-sm">
            <span className="font-medium text-foreground">EdgeQuake</span>
            {message.timestamp ? (
              <span className="text-xs text-muted-foreground">
                {formatRelativeTime(message.timestamp, i18n.language)}
              </span>
            ) : null}
            {sessionId ? (
              <span className="sr-only">{sessionId}</span>
            ) : null}
          </div>

          {showStage ? (
            <StageTimeline
              stage={stage as StreamStage}
              detail={stageDetail}
              sourceCount={message.context?.chunks?.length}
            />
          ) : null}

          {hasThinking ? (
            <ReasoningPanel
              thinking={thinkingBlocks}
              thinkingTimeMs={message.thinkingTimeMs}
              isLive={panelLive}
              isExpanded={thinkingExpanded}
              onToggle={() => dispatchReasoning({ type: "user_toggle" })}
            />
          ) : null}

          {(displayContent || (message.isStreaming && !showStage)) && (
            <div
              className={cn(
                "min-w-0",
                // Unboxed answer — no card chrome (Q19)
              )}
            >
              {displayContent ? (
                <div className="break-words overflow-wrap-anywhere hyphens-auto prose-chat">
                  <StreamingMarkdownRenderer
                    content={displayContent}
                    isStreaming={message.isStreaming}
                    className=""
                    resolveCitation={resolveCitation}
                  />
                </div>
              ) : message.isStreaming ? (
                <p className="text-sm text-muted-foreground">
                  {t("query.generating", "Generating response...")}
                </p>
              ) : null}
            </div>
          )}

          {message.context ? (
            <SourceChips
              context={message.context}
              onOpenSources={() => setSourcesOpenSignal((n) => n + 1)}
            />
          ) : null}

          {message.stopped ? (
            <p
              className="text-xs text-muted-foreground"
              data-testid="query-stopped-banner"
            >
              {t("query.stoppedBanner", "Stopped — partial answer kept")}
            </p>
          ) : null}

          {showMetadata && !message.isStreaming && displayContent ? (
            <MessageActions
              copied={copied}
              onCopy={handleCopy}
              onRegenerate={onRegenerate}
              onShowOnGraph={
                hasGraphEntities ? handleShowOnGraph : undefined
              }
              onFeedback={handleFeedback}
              feedback={feedback}
              isLast={isLast}
              isVisible={!!isLast}
              stopped={message.stopped}
              onContinue={onContinue}
            />
          ) : null}

          {/* Sources panel — reserve only ~2.5rem when collapsed (Q14) */}
          {!message.isStreaming && displayContent ? (
            <div
              className={cn(
                "mt-1",
                message.context ? "min-h-10" : "min-h-0",
              )}
              data-testid="spec100-query-citations-slot"
            >
              {message.context ? (
                <SourceCitations
                  context={message.context}
                  openSignal={sourcesOpenSignal}
                  onEntityClick={(entityId) => {
                    router.push(
                      `/graph?entity=${encodeURIComponent(entityId)}`,
                    );
                  }}
                  onDocumentClick={(opts) =>
                    openSource(locationFromDocumentClick(opts))
                  }
                  onExploreGraph={(entityLabels) => {
                    const params = new URLSearchParams();
                    if (entityLabels.length > 0) {
                      params.set("entities", entityLabels.join(","));
                      params.set("focus", entityLabels[0]);
                    }
                    router.push(
                      `/graph${params.toString() ? `?${params}` : ""}`,
                    );
                  }}
                />
              ) : null}
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
});

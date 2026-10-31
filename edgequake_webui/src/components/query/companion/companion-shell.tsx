/**
 * SPEC-157 W1 — Frame for the Query companion pane.
 *
 *   side  : resizable column between chat and history (LAW-157-2/6)
 *   sheet : bottom sheet when the row is too narrow for chat + pane
 *
 * One pane at a time (Source XOR Graph). Rendering is delegated to
 * `SourcePane` / `GraphPane`; this file only owns the chrome, a11y and the
 * single resize handle.
 */
"use client";

import { Button } from "@/components/ui/button";
import { ResizablePanel } from "@/components/ui/resizable-panel";
import { studioAnswerHref } from "@/hooks/use-open-answer-graph";
import { useReturnFocus } from "@/hooks/use-return-focus";
import type { CompanionLayout } from "@/lib/query/companion-layout";
import { locationToDocumentHref } from "@/lib/query/companion-pane";
import { studioEntityHref } from "@/lib/query/neighborhood-graph";
import type { QueryMessage } from "@/lib/query/query-interface-types";
import { cn } from "@/lib/utils";
import { useWorkspaceCapabilities } from "@/hooks/use-workspace-capabilities";
import { useCompanionPaneStore } from "@/stores/use-companion-pane-store";
import type { GraphNode } from "@/types/graph";
import { ExternalLink, X } from "lucide-react";
import Link from "next/link";
import { useId, useState, type KeyboardEvent, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import {
  CompanionTabs,
  companionPanelId,
  companionTabId,
} from "./companion-tabs";
import { EntityNeighborhoodPane } from "./entity-neighborhood-pane";
import { GraphPane } from "./graph-pane";
import { SourcePane } from "./source-pane";
import { WorkspaceGraphPane } from "./workspace-graph-pane";

interface CompanionShellProps {
  layout: CompanionLayout;
  messages: QueryMessage[];
  isMessagesLoading: boolean;
  onQuote: (text: string) => void;
  /** SPEC-159: Ask handoff — always starts a new conversation. */
  onAskEntity?: (node: GraphNode) => void;
  /** Where focus lands when the opener no longer exists. */
  onRestoreFocus: () => void;
}

function Announcer({ text }: { text: string }) {
  return (
    <div className="sr-only" role="status" aria-live="polite" aria-atomic>
      {text}
    </div>
  );
}

export function CompanionShell({
  layout,
  messages,
  isMessagesLoading,
  onQuote,
  onAskEntity,
  onRestoreFocus,
}: CompanionShellProps) {
  const { t } = useTranslation();
  const idPrefix = useId();
  const target = useCompanionPaneStore((s) => s.target);
  const lastSource = useCompanionPaneStore((s) => s.lastSource);
  const close = useCompanionPaneStore((s) => s.close);
  const switchTo = useCompanionPaneStore((s) => s.switchTo);
  const setWidth = useCompanionPaneStore((s) => s.setWidth);
  // SPEC-180: a workspace without a graph never mounts a graph pane.
  const showsGraph = useWorkspaceCapabilities().shows_graph_ui;

  const open = target.kind !== "none";
  useReturnFocus(open, onRestoreFocus);

  // Announce "closed" only after it was open (adjust state during render).
  const [prevOpen, setPrevOpen] = useState(open);
  const [justClosed, setJustClosed] = useState(false);
  if (prevOpen !== open) {
    setPrevOpen(open);
    setJustClosed(!open);
  }

  const announcement = open
    ? target.kind === "pdf"
      ? t("query.companion.announceOpenSource", "Source opened beside the chat")
      : target.entityId
        ? t(
            "query.companion.announceOpenNeighborhood",
            "Entity neighbourhood opened beside the chat",
          )
        : target.messageId
          ? t(
              "query.companion.announceOpenGraph",
              "Answer graph opened beside the chat",
            )
          : t(
              "query.companion.announceOpenWorkspaceGraph",
              "Knowledge graph opened beside the chat",
            )
    : justClosed
      ? t("query.companion.announceClosed", "Pane closed")
      : "";

  if (target.kind === "none" || (target.kind === "graph" && !showsGraph)) {
    return <Announcer text={announcement} />;
  }

  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape" && !e.defaultPrevented) {
      e.stopPropagation();
      close();
    }
  };

  const fullPageHref =
    target.kind === "pdf" && target.source
      ? locationToDocumentHref(target.source)
      : target.entityId
        ? studioEntityHref(target.entityId)
        : target.messageId
          ? studioAnswerHref(target.messageId)
          : "/graph";

  const body: ReactNode =
    target.kind === "pdf" && target.source ? (
      <SourcePane location={target.source} onQuote={onQuote} />
    ) : target.kind === "graph" && target.entityId ? (
      <EntityNeighborhoodPane
        entityId={target.entityId}
        onAskEntity={onAskEntity}
      />
    ) : target.kind === "graph" && target.messageId ? (
      <GraphPane
        messageId={target.messageId}
        messages={messages}
        isLoading={isMessagesLoading}
        onAskEntity={onAskEntity}
      />
    ) : target.kind === "graph" ? (
      <WorkspaceGraphPane onAskEntity={onAskEntity} />
    ) : null;

  const frame = (
    <aside
      aria-label={t("query.companion.title", "Companion pane")}
      data-testid="query-companion"
      data-companion-kind={target.kind}
      data-companion-mode={layout.mode}
      onKeyDown={onKeyDown}
      className={cn(
        "flex min-h-0 flex-col bg-background",
        layout.mode === "side"
          ? "h-full w-full border-l"
          : "fixed inset-x-0 bottom-0 z-40 h-[62vh] rounded-t-xl border-t shadow-2xl",
      )}
    >
      <header className="flex h-11 shrink-0 items-center gap-2 border-b bg-card/60 px-3">
        <CompanionTabs
          idPrefix={idPrefix}
          active={target.kind}
          available={{ pdf: Boolean(lastSource), graph: showsGraph }}
          showGraph={showsGraph}
          onSelect={switchTo}
        />
        <div className="flex-1" />
        {fullPageHref ? (
          <Button
            asChild
            variant="ghost"
            size="icon"
            className="h-7 w-7"
          >
            <Link
              href={fullPageHref}
              aria-label={
                target.kind === "pdf"
                  ? t("query.companion.openFullPage", "Open full page")
                  : t("query.companion.openStudio", "Open in Graph Studio")
              }
              title={
                target.kind === "pdf"
                  ? t("query.companion.openFullPage", "Open full page")
                  : t("query.companion.openStudio", "Open in Graph Studio")
              }
              data-testid="companion-open-full"
            >
              <ExternalLink className="h-3.5 w-3.5" />
            </Link>
          </Button>
        ) : null}
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="h-7 w-7"
          onClick={close}
          aria-label={t("query.companion.close", "Close pane")}
          title={t("query.companion.close", "Close pane")}
          data-testid="companion-close"
        >
          <X className="h-4 w-4" />
        </Button>
      </header>
      <div
        role="tabpanel"
        id={companionPanelId(idPrefix)}
        aria-labelledby={companionTabId(idPrefix, target.kind)}
        className="min-h-0 flex-1"
      >
        {body}
      </div>
    </aside>
  );

  return (
    <>
      <Announcer text={announcement} />
      {layout.mode === "side" ? (
        <ResizablePanel
          side="right"
          width={layout.widthPx}
          defaultWidth={layout.widthPx}
          minWidth={layout.minPx}
          maxWidth={layout.maxPx}
          onWidthChange={setWidth}
          ariaLabel={t("query.companion.resize", "Resize companion pane")}
          className="flex"
        >
          {frame}
        </ResizablePanel>
      ) : (
        frame
      )}
    </>
  );
}

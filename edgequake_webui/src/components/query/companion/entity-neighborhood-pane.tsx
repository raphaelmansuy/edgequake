/**
 * SPEC-159 — Companion graph for Ask: 1-hop neighborhood of a studio entity.
 * Isolated from Graph Studio store (LAW-159-8).
 */
"use client";

import { EmbeddedAnswerGraph } from "@/components/graph/embedded-answer-graph";
import { Button } from "@/components/ui/button";
import { ApiRequestError } from "@/lib/api/client";
import { getEntityNeighborhood } from "@/lib/api/edgequake/graph";
import {
  buildNeighborhoodGraphModel,
  resolveSeedNodeId,
  studioEntityHref,
} from "@/lib/query/neighborhood-graph";
import type { GraphNode } from "@/types/graph";
import { AlertCircle, ExternalLink, Loader2, Network } from "lucide-react";
import Link from "next/link";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { AnswerGraphList } from "./answer-graph-list";
import { CompanionViewAskBar } from "./companion-view-ask-bar";
import { GraphNodeCard } from "./graph-node-card";
import { PaneNotice } from "./pane-notice";

type GraphView = "canvas" | "list";
type LoadState =
  | { status: "loading" }
  | { status: "error"; notFound: boolean; message: string }
  | {
      status: "ready";
      model: ReturnType<typeof buildNeighborhoodGraphModel>;
      seedId: string | null;
    };

interface EntityNeighborhoodPaneProps {
  entityId: string;
  onAskEntity?: (node: GraphNode) => void;
}

export function EntityNeighborhoodPane({
  entityId,
  onAskEntity,
}: EntityNeighborhoodPaneProps) {
  const { t } = useTranslation();
  // The last completed load, tagged with the entity it belongs to.
  //
  // WHY the tag instead of resetting to `loading` inside the effect: calling `setState` in an
  // effect body schedules a second render on every entity change, which the `set-state-in-effect`
  // rule rightly flags as a cascade. Storing which entity the result belongs to makes "loading"
  // a *derived* fact — `loaded?.entityId !== entityId` — so switching entities shows the loader
  // during the same render, with no extra pass and no flicker of the previous entity's graph.
  const [loaded, setLoaded] = useState<
    { entityId: string; state: Exclude<LoadState, { status: "loading" }> } | null
  >(null);

  useEffect(() => {
    let cancelled = false;
    void getEntityNeighborhood(entityId, 1)
      .then((res) => {
        if (cancelled) return;
        const model = buildNeighborhoodGraphModel(
          res.nodes ?? [],
          res.edges ?? [],
          entityId,
        );
        const seedId = resolveSeedNodeId(model.nodes, entityId);
        setLoaded({ entityId, state: { status: "ready", model, seedId } });
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        const notFound =
          err instanceof ApiRequestError && err.status === 404;
        setLoaded({
          entityId,
          state: {
            status: "error",
            notFound,
            message:
              err instanceof Error
                ? err.message
                : t("query.companion.neighborhoodFailed", "Couldn't load neighbourhood"),
          },
        });
      });
    return () => {
      cancelled = true;
    };
  }, [entityId, t]);

  const state: LoadState =
    loaded?.entityId === entityId ? loaded.state : { status: "loading" };

  if (state.status === "loading") {
    return (
      <PaneNotice
        icon={Loader2}
        title={t("query.companion.loading", "Loading…")}
        testId="companion-entity-loading"
      />
    );
  }

  if (state.status === "error") {
    return (
      <PaneNotice
        icon={AlertCircle}
        title={
          state.notFound
            ? t(
                "query.companion.entityNotFound",
                "Entity not found in this workspace",
              )
            : t(
                "query.companion.neighborhoodFailed",
                "Couldn't load neighbourhood",
              )
        }
        description={state.notFound ? undefined : state.message}
        testId="companion-entity-error"
      />
    );
  }

  if (state.model.nodes.length === 0) {
    return (
      <PaneNotice
        icon={Network}
        title={t(
          "query.companion.emptyNeighborhood",
          "No neighbours within 1 hop",
        )}
        description={t(
          "query.companion.emptyNeighborhoodHint",
          "This entity has no connected nodes in the knowledge graph yet.",
        )}
        testId="companion-entity-empty"
      />
    );
  }

  return (
    <NeighborhoodView
      key={entityId}
      entityId={entityId}
      model={state.model}
      initialSelectedId={state.seedId}
      onAskEntity={onAskEntity}
    />
  );
}

function NeighborhoodView({
  entityId,
  model,
  initialSelectedId,
  onAskEntity,
}: {
  entityId: string;
  model: ReturnType<typeof buildNeighborhoodGraphModel>;
  initialSelectedId: string | null;
  onAskEntity?: (node: GraphNode) => void;
}) {
  const { t } = useTranslation();
  const [view, setView] = useState<GraphView>("canvas");
  const [selectedId, setSelectedId] = useState<string | null>(
    initialSelectedId,
  );
  const selected = useMemo(
    () => model.nodes.find((n) => n.id === selectedId) ?? null,
    [model.nodes, selectedId],
  );

  const list = (
    <AnswerGraphList
      model={model}
      selectedNodeId={selectedId}
      onSelect={setSelectedId}
    />
  );

  return (
    <div
      className="flex h-full min-h-0 flex-col"
      data-testid="companion-entity-graph"
      data-entity-id={entityId}
    >
      <div className="flex shrink-0 items-center gap-2 border-b px-3 py-2">
        <p
          className="min-w-0 flex-1 truncate text-xs text-muted-foreground"
          data-testid="companion-entity-summary"
        >
          {t(
            "query.companion.neighborhoodSummary",
            "{{nodes}} entities · {{edges}} relationships · 1 hop",
            {
              nodes: model.nodes.length,
              edges: model.edges.length,
            },
          )}
        </p>
        <CompanionViewAskBar
          view={view}
          onViewChange={setView}
          onAsk={
            onAskEntity
              ? () => {
                  const target =
                    selected ??
                    model.nodes.find((n) => n.id === initialSelectedId) ??
                    model.nodes[0];
                  if (target) onAskEntity(target);
                }
              : undefined
          }
          testIdPrefix="companion-entity-view"
        />
      </div>

      <div className="min-h-0 flex-1">
        {view === "canvas" ? (
          <EmbeddedAnswerGraph
            nodes={model.nodes}
            edges={model.edges}
            selectedNodeId={selectedId}
            onSelect={setSelectedId}
            fallback={list}
          />
        ) : (
          list
        )}
      </div>

      {selected ? (
        <GraphNodeCard
          node={selected}
          isAnswerNode={model.answerNodeIds.includes(selected.id)}
          onClose={() => setSelectedId(null)}
          onAskAboutThis={onAskEntity}
        />
      ) : null}

      <div className="flex shrink-0 items-center justify-end border-t bg-muted/30 px-3 py-2">
        <Button asChild variant="outline" size="sm" className="h-7 gap-1.5 text-xs">
          <Link
            href={studioEntityHref(entityId)}
            data-testid="companion-open-studio"
          >
            <ExternalLink className="h-3 w-3" aria-hidden />
            {t("query.companion.openStudio", "Open in Graph Studio")}
          </Link>
        </Button>
      </div>
    </div>
  );
}

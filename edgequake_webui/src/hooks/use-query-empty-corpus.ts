"use client";

/**
 * SPEC-183 / SPEC-184 — what the empty Query screen should suggest, given what this corpus holds.
 *
 * The default suggestions used to be one hardcoded list of text-document questions ("What does
 * the handbook say about refunds?"), shown to every Normal workspace. On a workspace of twenty
 * photographs all four were unanswerable, and clicking one produced an honest refusal — so the
 * UI asked a question its own data could not answer.
 *
 * Counting needs the document list, because the stats endpoint reports totals only. The list is
 * already cached by the documents screen, so the cost is bounded: one page, no polling, and the
 * suggestion text is cosmetic — if the request fails, the text corpus copy is used, which is the
 * pre-existing behaviour.
 *
 * SPEC-184 adds two things to that shape:
 *   - `documentCount` from the server's stats, which is the only way to distinguish "this
 *     workspace is empty" from "the list has not loaded". An empty workspace has no answerable
 *     question and must show none.
 *   - the graph's own vocabulary (top entities, declared relation types) for a graph workspace,
 *     because a question about a knowledge graph that names no entity in it is exactly the
 *     unanswerable-promise defect this file exists to prevent.
 *
 * Every query below is scoped to the selected workspace. The entity query previously used a
 * workspace-independent cache key, so switching workspace could surface another workspace's
 * entities as suggestions.
 */

import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api/client";
import { useWorkspaceCapabilities } from "@/hooks/use-workspace-capabilities";
import type { CorpusShape } from "@/lib/query/query-empty-copy";
import { getWorkspaceStats } from "@/lib/api/edgequake/workspaces";
import { useSelectedWorkspace, useTenantStore } from "@/stores/use-tenant-store";

interface DocumentRow {
  source_type?: string | null;
  content_type?: string | null;
  file_name?: string | null;
  title?: string | null;
}

/**
 * Document family, mirroring the server's `doc_kind` (migration 225).
 *
 * WHY no `office`/`other` members: Office is a v1 non-goal (LAW-121-8) and nothing in the ingest
 * path converts a word-processor file, so a row can only reach those shapes if the server refused
 * it. Classification is about what this corpus *holds*, and these are the three families the
 * product can actually store. `other` is the honest fallback for an unrecognised row, not a family
 * with questions attached to it.
 */
export type DocumentFamily = "image" | "pdf" | "text" | "other";

/**
 * Classify one document row.
 *
 * WHY `source_type` first: the upload path records the family it actually ingested as
 * (`source_type: "image"`), which is more reliable than re-deriving it from a MIME string the
 * same request may not have carried. The MIME and the extension are fallbacks for older rows.
 */
export function documentFamily(row: DocumentRow): DocumentFamily {
  const source = (row.source_type ?? "").toLowerCase();
  if (source === "image") return "image";
  if (source === "pdf") return "pdf";
  if (source === "markdown" || source === "text") return "text";

  const mime = (row.content_type ?? "").toLowerCase();
  if (mime.startsWith("image/")) return "image";
  if (mime === "application/pdf") return "pdf";
  if (mime.startsWith("text/")) return "text";

  const name = `${row.file_name ?? ""} ${row.title ?? ""}`.toLowerCase();
  if (/\.(png|jpe?g|gif|webp|bmp|tiff?)\b/.test(name)) return "image";
  if (/\.pdf\b/.test(name)) return "pdf";
  if (/\.(md|markdown|txt|csv|json)\b/.test(name)) return "text";
  return "other";
}

/** True when one document row is a picture. Kept for callers that only need the boolean. */
export function isImageDocument(row: DocumentRow): boolean {
  return documentFamily(row) === "image";
}

/** How a page of documents maps onto the empty state's needs. Pure, so it is testable. */
export function corpusShapeFromDocuments(
  hasGraph: boolean,
  embedsImages: boolean,
  rows: readonly DocumentRow[],
  documentCount?: number,
): CorpusShape {
  const count = (family: DocumentFamily) =>
    rows.filter((row) => documentFamily(row) === family).length;
  return {
    hasGraph,
    embedsImages,
    documentCount,
    chunkCount: rows.length,
    imageDocumentCount: count("image"),
    pdfDocumentCount: count("pdf"),
    textDocumentCount: count("text"),
    sampleTitles: rows
      .map((row) => row.title ?? row.file_name ?? "")
      .filter((t) => t.length > 0)
      .slice(0, 10),
  };
}

interface SearchNodeHit {
  label?: string;
  name?: string;
  degree?: { in?: number; out?: number; total?: number } | number;
}

/**
 * The labels of the workspace's best-connected entities, for a graph suggestion to name.
 *
 * WHY this shape: `GET /graph/nodes/search` returns `{ nodes, edges, total_matches }`, not
 * `{ items }`. The previous caller read `items`, so it always found nothing and the entity
 * suggestions silently never rendered — the empty state in the bug report was that path failing.
 *
 * WHY degree-ordered and capped: a suggestion must name something the graph really holds, and the
 * most connected entities are the ones whose neighbourhood a first question can actually answer. An
 * empty or failed response yields `[]`, which makes the entity templates unfillable — and an
 * unfillable template is refused, so no question invents a subject.
 */
async function fetchSampleEntities(): Promise<string[]> {
  const res = await api.get<{ nodes?: SearchNodeHit[] }>("/graph/nodes/search?q=&limit=12");
  const degreeOf = (hit: SearchNodeHit): number => {
    if (typeof hit.degree === "number") return hit.degree;
    return hit.degree?.total ?? 0;
  };
  return (res.nodes ?? [])
    .slice()
    .sort((a, b) => degreeOf(b) - degreeOf(a))
    .map((hit) => hit.label ?? hit.name ?? "")
    .filter((label) => label.trim().length > 0)
    .slice(0, 4);
}

export function useQueryEmptyCorpus(enabled: boolean): CorpusShape {
  const caps = useWorkspaceCapabilities();
  const workspace = useSelectedWorkspace();
  const workspaceId = useTenantStore((s) => s.selectedWorkspaceId);

  const { data: documents } = useQuery({
    queryKey: ["query-empty-corpus", workspaceId],
    queryFn: async () => {
      const res = await api.get<{ documents?: DocumentRow[]; items?: DocumentRow[] }>(
        "/documents?limit=100",
      );
      return res.documents ?? res.items ?? [];
    },
    enabled: enabled && Boolean(workspaceId),
    staleTime: 60_000,
  });

  const { data: stats } = useQuery({
    queryKey: ["query-empty-stats", workspaceId],
    queryFn: () => getWorkspaceStats(workspaceId!).catch(() => null),
    enabled: enabled && Boolean(workspaceId),
    staleTime: 60_000,
  });

  // Scoped by workspace id: the previous key was constant, so one workspace's entities could be
  // suggested inside another. Only a graph workspace has entities to name.
  const { data: entities } = useQuery({
    queryKey: ["query-empty-entities", workspaceId],
    queryFn: () => fetchSampleEntities().catch(() => [] as string[]),
    enabled: enabled && Boolean(workspaceId) && caps.shows_graph_ui,
    staleTime: 60_000,
  });

  const shape = corpusShapeFromDocuments(
    caps.shows_graph_ui,
    caps.embeds_images,
    documents ?? [],
    stats?.document_count,
  );

  return {
    ...shape,
    // The workspace carries the schema it was created with, so a relation-shaped question uses
    // the operator's own vocabulary rather than a guessed domain (SPEC-114).
    sampleEntities: entities ?? [],
    sampleRelations: workspace?.relation_types ?? [],
    sampleTitles: shape.sampleTitles,
  };
}

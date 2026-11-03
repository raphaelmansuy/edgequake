/**
 * SPEC-183 — empty-state copy and corpus-derived suggestions for the Query screen.
 *
 * ## The problem this solves
 *
 * A classical RAG corpus is **not** text. It is whatever the operator uploaded: handbooks and
 * spreadsheets, scanned PDFs with page numbers, photographs with no text at all, or — most
 * often — a mixture. The empty state used to offer one hardcoded list of text questions
 * ("What does the handbook say about refunds?") to every Normal workspace, so a workspace of
 * twenty photographs was greeted with four questions none of its documents could answer.
 *
 * The first fix branched on "are most documents pictures?". That is better, but it still treats
 * the corpus as one thing: a workspace holding a brand handbook *and* its product photographs got
 * a single-kind greeting and could never see a question about the manual.
 *
 * ## The rule now
 *
 * Every suggestion **declares the capability it needs**. The corpus reports which capabilities it
 * actually has, and suggestions are drawn from the pools whose capability is present, interleaved
 * so a mixed corpus shows its variety instead of its majority. A suggestion is only ever printed
 * if the corpus can keep the promise it makes.
 *
 * Pure and data-driven, so the rule is a unit test rather than a screenshot.
 */

import type { QueryMode } from "@/types/query";

export interface QueryEmptyCopy {
  title: string;
  description: string;
  suggestions: string[];
  /** i18n keys, so the component never branches on which copy it got. */
  keys: { title: string; description: string; suggestions: string; subtitle: string };
  /** Subtitle under the page title. */
  subtitle: string;
}

/**
 * What a document can be asked about. Derived from the file's own facts, never from the
 * workspace name (LAW-180-2) and never from the profile.
 */
export type CorpusCapability = "text" | "pdf" | "image" | "office";

/** How many suggestions the empty state shows. Four fit the 2×2 grid. */
export const SUGGESTION_SLOTS = 4;

/** What the corpus holds, as far as the empty state needs to know. */
export interface CorpusShape {
  /** Does the workspace build a knowledge graph? (`shows_graph_ui`) */
  hasGraph: boolean;
  /** Does it embed pictures? (`embeds_images`) — false means pictures cannot be *searched by picture*. */
  embedsImages?: boolean;
  /** How many chunks exist. Zero means "nothing uploaded yet". */
  chunkCount?: number;
  /** Documents by family. Each count is what makes the matching pool available. */
  imageDocumentCount?: number;
  pdfDocumentCount?: number;
  textDocumentCount?: number;
  officeDocumentCount?: number;
  /** A few real titles, so one suggestion can name something the corpus actually contains. */
  sampleTitles?: string[];
}

/** One candidate question, and the capability that makes it answerable. */
export interface CorpusSuggestion {
  text: string;
  needs: CorpusCapability;
  /** i18n key for the text, so translation does not depend on the English string. */
  key: string;
}

/**
 * The suggestion pools, one per capability.
 *
 * WHY these questions: each one names the mechanism that makes it answerable — "passages" and
 * "quote" for text, "page" for a PDF, "picture"/"look like" for an image. A reader learns what
 * the workspace can do from the questions it is offered, so a vague suggestion wastes the slot.
 */
const POOLS: Record<CorpusCapability, CorpusSuggestion[]> = {
  text: [
    {
      text: "What does {{topic}} say about refunds?",
      key: "query.corpusSuggest.text.topic",
      needs: "text",
    },
    {
      text: "Find the passage that mentions deadlines",
      key: "query.corpusSuggest.text.deadlines",
      needs: "text",
    },
    {
      text: "Which documents cover onboarding?",
      key: "query.corpusSuggest.text.onboarding",
      needs: "text",
    },
  ],
  pdf: [
    {
      text: "Quote the section about security requirements",
      key: "query.corpusSuggest.pdf.quote",
      needs: "pdf",
    },
    {
      text: "Which page explains the approval process?",
      key: "query.corpusSuggest.pdf.page",
      needs: "pdf",
    },
    {
      text: "Summarise the longest document",
      key: "query.corpusSuggest.pdf.summarise",
      needs: "pdf",
    },
  ],
  image: [
    {
      text: "Which pictures are in this workspace?",
      key: "query.corpusSuggest.image.list",
      needs: "image",
    },
    {
      text: "Find a picture with a body of water",
      key: "query.corpusSuggest.image.water",
      needs: "image",
    },
    {
      text: "Show me the pictures with a person in them",
      key: "query.corpusSuggest.image.person",
      needs: "image",
    },
  ],
  office: [
    {
      text: "What does the spreadsheet say about totals?",
      key: "query.corpusSuggest.office.totals",
      needs: "office",
    },
    {
      text: "Summarise the rows in the latest table",
      key: "query.corpusSuggest.office.summarise",
      needs: "office",
    },
  ],
};

/** Which capabilities the corpus actually has, in the order a mixed corpus should show them. */
export function capabilitiesOf(shape: CorpusShape): CorpusCapability[] {
  const counts: Array<[CorpusCapability, number]> = [
    ["text", shape.textDocumentCount ?? 0],
    ["pdf", shape.pdfDocumentCount ?? 0],
    ["office", shape.officeDocumentCount ?? 0],
    // A picture is only *askable* when the workspace embeds images; a text-only Normal workspace
    // cannot store one, so offering picture questions there would be the original bug again.
    ["image", shape.embedsImages === true ? (shape.imageDocumentCount ?? 0) : 0],
  ];
  return counts.filter(([, n]) => n > 0).map(([cap]) => cap);
}

/**
 * Compose the suggestions for a corpus.
 *
 * Round-robin across the capabilities present, so a corpus that is 90% photographs and 10%
 * handbooks still offers a handbook question in its four slots. Ordering within a pool is stable,
 * so the list does not reshuffle between renders.
 *
 * When the corpus has nothing to say about itself — no counts yet, or an empty workspace — the
 * text pool is returned, because it is the pre-existing behaviour and the "upload something"
 * hint beside it carries the real message.
 */
export function suggestionsFor(
  shape: CorpusShape,
  slots: number = SUGGESTION_SLOTS,
): CorpusSuggestion[] {
  const caps = capabilitiesOf(shape);
  const pools = (caps.length > 0 ? caps : (["text"] as CorpusCapability[])).map((cap) => [
    ...POOLS[cap],
  ]);
  const topic = topicFromTitles(shape.sampleTitles);

  const out: CorpusSuggestion[] = [];
  let round = 0;
  // Bounded: at most one pass per pool, and pools have three entries at most.
  while (out.length < slots && round < 4) {
    for (const pool of pools) {
      const next = pool.shift();
      if (next && out.length < slots) {
        out.push({
          ...next,
          // Never let a placeholder reach the screen: the i18n string is what actually renders,
          // and it carries `{{topic}}` too, so the substitution has to happen here, on the text
          // the component falls back to.
          text: next.text.replace("{{topic}}", topic),
        });
      }
    }
    round += 1;
  }
  return out;
}

/**
 * The topic word for a suggestion that names one.
 *
 * WHY exported and applied by the component too: `suggestionsFor` substitutes the placeholder in
 * the English fallback, but the rendered string comes from `t(key, fallback)`, whose value also
 * contains `{{topic}}`. Both call sites must use this one function or the screen can show a raw
 * placeholder — which is exactly what happened the first time.
 */
export function topicForCorpus(shape: CorpusShape): string {
  return topicFromTitles(shape.sampleTitles);
}

/**
 * A neutral noun for "{{topic}}" when the corpus has no usable title.
 *
 * WHY not the first filename: a workspace of photographs would produce "What does 05-alpine-lake
 * say about refunds?", which is nonsense. A real title is only used when it reads like prose.
 */
export function topicFromTitles(titles: readonly string[] | undefined): string {
  const candidate = (titles ?? [])
    .map((t) => t.trim())
    .find((t) => t.length > 0 && /^[A-Za-z][A-Za-z0-9 _-]{2,}$/.test(t) && !/\d{2}-/.test(t));
  return candidate ? candidate.replace(/\.[a-z0-9]+$/i, "").toLowerCase() : "the handbook";
}

interface CopyHead {
  title: string;
  description: string;
  subtitle: string;
  keys: QueryEmptyCopy["keys"];
}

const TEXT_HEAD: CopyHead = {
  title: "Ask about your documents",
  description:
    "I find the passages that answer your question, quote them and point to the page.",
  subtitle: "Ask questions about your documents",
  keys: {
    title: "query.normalEmptyTitle",
    description: "query.normalEmptyDescription",
    suggestions: "query.normalSuggestions",
    subtitle: "query.normalSubtitle",
  },
};

const IMAGE_ONLY_HEAD: CopyHead = {
  title: "Search your pictures",
  description:
    "I match pictures by what they show, and text by what it says. Ask in words, or attach a picture to find similar ones.",
  subtitle: "Search pictures by description, or by another picture",
  keys: {
    title: "query.normalImagesEmptyTitle",
    description: "query.normalImagesEmptyDescription",
    suggestions: "query.normalImagesSuggestions",
    subtitle: "query.normalImagesSubtitle",
  },
};

/**
 * Pictures *and* text together.
 *
 * WHY a third head: the two single-kind sentences are both wrong for a mixed corpus. "I find the
 * passages… point to the page" ignores the photographs; "Search your pictures" implies there is
 * nothing else. A mixed workspace is the common case, and it deserves a sentence that mentions
 * both without promising a page number for a JPEG.
 */
const MIXED_HEAD: CopyHead = {
  title: "Search your documents and pictures",
  description:
    "I answer from the text you uploaded and match pictures by what they show. Ask in words, cite a passage, or attach a picture to find similar ones.",
  subtitle: "Documents and pictures, in one search",
  keys: {
    title: "query.normalMixedEmptyTitle",
    description: "query.normalMixedEmptyDescription",
    suggestions: "query.normalMixedSuggestions",
    subtitle: "query.normalMixedSubtitle",
  },
};

const RAG_COPY: QueryEmptyCopy = {
  title: "Ask about your knowledge graph",
  description:
    "I can help you explore entities, find connections, and uncover insights from your documents.",
  suggestions: [
    "What are the main entities in my knowledge graph?",
    "Summarize the key relationships between documents",
    "Find connections between people and organizations",
    "What topics are covered in my documents?",
  ],
  keys: {
    title: "query.emptyTitle",
    description: "query.emptyDescription",
    suggestions: "query.suggestions",
    subtitle: "query.subtitle",
  },
  subtitle: "Ask questions about your knowledge graph",
};

const CHAT_COPY: QueryEmptyCopy = {
  title: "Chat with your assistant",
  description:
    "General conversation without document or graph retrieval. Follow-ups use recent chat history.",
  suggestions: [
    "Help me brainstorm ideas",
    "Explain a concept in simple terms",
    "What should I consider before starting a project?",
    "Summarize the trade-offs of two approaches",
  ],
  keys: {
    title: "query.chatEmptyTitle",
    description: "query.chatEmptyDescription",
    suggestions: "query.chatSuggestions",
    subtitle: "query.chatSubtitle",
  },
  subtitle: "General chat — no knowledge graph retrieval",
};

/** Which head sentence a corpus earns. */
function headFor(shape: CorpusShape): CopyHead {
  const textish =
    (shape.textDocumentCount ?? 0) + (shape.pdfDocumentCount ?? 0) + (shape.officeDocumentCount ?? 0);
  const pictures = shape.embedsImages === true ? (shape.imageDocumentCount ?? 0) : 0;
  if (pictures > 0 && textish > 0) return MIXED_HEAD;
  if (pictures > 0) return IMAGE_ONLY_HEAD;
  return TEXT_HEAD;
}

/**
 * The copy for the current mode and corpus.
 *
 * Precedence, and the reason for each step:
 *   1. Chat mode has no retrieval at all, so the corpus is irrelevant.
 *   2. A graph workspace answers about entities, whatever the file kinds.
 *   3. Everything else is a Normal workspace: the head names the kinds it actually holds, and
 *      the suggestions are drawn from the pools those kinds unlock.
 */
export function getQueryEmptyCopy(
  mode: QueryMode = "mix",
  corpus: CorpusShape | boolean = { hasGraph: true },
): QueryEmptyCopy {
  if (mode === "bypass") return CHAT_COPY;
  // Back-compat: the old call shape was `(mode, hasGraph)`.
  const shape: CorpusShape = typeof corpus === "boolean" ? { hasGraph: corpus } : corpus;
  if (shape.hasGraph) return RAG_COPY;

  const head = headFor(shape);
  return {
    title: head.title,
    description: head.description,
    subtitle: head.subtitle,
    keys: head.keys,
    suggestions: suggestionsFor(shape).map((s) => s.text),
  };
}

/** Suggestion keys, so the component can translate each one by its own key. */
export function getQueryEmptySuggestionKeys(
  mode: QueryMode,
  corpus: CorpusShape | boolean,
): string[] {
  if (mode === "bypass") return [];
  const shape: CorpusShape = typeof corpus === "boolean" ? { hasGraph: corpus } : corpus;
  if (shape.hasGraph) return [];
  return suggestionsFor(shape).map((s) => s.key);
}

export function isChatQueryMode(mode: QueryMode): boolean {
  return mode === "bypass";
}

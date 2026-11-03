/**
 * SPEC-183 / SPEC-184 — empty-state copy and corpus-derived suggestions for the Query screen.
 *
 * ## The problem this solves
 *
 * A default question is not copy. It is a **promise about this corpus**: click it and the
 * workspace must be able to keep that promise. A workspace of twenty photographs greeted with
 * "What does the handbook say about refunds?" is asked a question its own data cannot answer, and
 * the first thing the product does is lie.
 *
 * ## The rule
 *
 * Every suggestion declares the capability it needs. The corpus reports which capabilities it
 * actually has, and suggestions are drawn from the pools those unlock, interleaved so a mixed
 * corpus shows its variety instead of its majority. A suggestion is printed only if the corpus can
 * keep the promise it makes.
 *
 * ## What this revision adds (three defects the first rule did not cover)
 *
 * 1. **Graph workspaces were exempt.** `getQueryEmptyCopy` returned a hardcoded list before any
 *    corpus logic ran, so the *default* workspace kind — and the one with a live entity graph —
 *    offered four sentences that could not mention a single entity it held. Graph is now composed
 *    from the same data as every other kind.
 * 2. **Subjects were fabricated.** "refunds", "onboarding", "body of water" and "a person" were
 *    invented, and the topic fallback literally returned the string `"the handbook"`. A template
 *    with an unfilled placeholder is now *refused* — three honest suggestions beat four where one
 *    invents its own premise.
 * 3. **Every question shared one mechanism.** Four paraphrases of "what do we know about X?"
 *    teach a reader nothing. Pools are keyed by **mechanism** (entity, relationship, theme,
 *    document; passage, page, table, picture) so the four slots demonstrate breadth.
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
 *
 * WHY there is no `office` here: Office (DOCX/XLSX/PPTX) is a **non-goal for v1** (LAW-121-8).
 * `DocKind::Office` exists in the classifier and the SQL `doc_kind` mapping, but nothing in the
 * ingest path converts a word-processor file — the supported product matrix is TXT, MD, JSON, PDF
 * and raster images. A pool for a family that cannot be ingested is a question nobody can answer,
 * which is precisely the defect this whole module exists to prevent. When an Office adapter lands,
 * it converts to Markdown and reuses text ingest, so the `text` pool already covers it.
 *
 * WHY `other` is gone too: `documentFamily` only reports `other` for a file whose MIME and
 * extension the server refuses, so a corpus can never be *composed* of them.
 */
export type CorpusCapability = "text" | "pdf" | "image";

/** A capability a suggestion needs. `graph` is a workspace capability, not a file family. */
export type SuggestionNeed = CorpusCapability | "graph";

/** How many suggestions the empty state shows. Four fit the 2×2 grid. */
export const SUGGESTION_SLOTS = 4;

/** Beyond this many documents, cross-document questions become answerable. */
const NASCENT_MAX = 3;

/**
 * How much the corpus can support.
 *
 * - `empty`   — a definite zero documents: **no** question is answerable, so none is shown.
 * - `unknown` — the counts have not arrived (or are loading): offer only questions that need no
 *               knowledge of the corpus, never a fabricated subject.
 * - `nascent` — one to three documents: inventory and per-document questions, no comparisons.
 * - `established` — enough material for cross-document questions to resolve.
 */
export type CorpusState = "empty" | "unknown" | "nascent" | "established";

/** What the corpus holds, as far as the empty state needs to know. */
export interface CorpusShape {
  /** Does the workspace build a knowledge graph? (`shows_graph_ui`) */
  hasGraph: boolean;
  /** Does it embed pictures? (`embeds_images`) — false means pictures cannot be *searched by picture*. */
  embedsImages?: boolean;
  /**
   * The server's `document_count`, when known.
   *
   * WHY separate from the family counts: those are derived from one page of the document list and
   * cannot distinguish "this workspace is empty" from "the list has not loaded". Only an explicit
   * zero may suppress the suggestions outright.
   */
  documentCount?: number;
  /** How many chunks exist. Zero means "nothing uploaded yet". */
  chunkCount?: number;
  /** Documents by family. Each count is what makes the matching pool available. */
  imageDocumentCount?: number;
  pdfDocumentCount?: number;
  textDocumentCount?: number;
  /** A few real titles, so a suggestion can name something the corpus actually contains. */
  sampleTitles?: string[];
  /** Real entity labels from the graph, so a graph suggestion can name one. */
  sampleEntities?: string[];
  /** Declared relation types (`relation_types`), so a graph question uses the workspace's own schema. */
  sampleRelations?: string[];
}

/** One candidate question, and the capability that makes it answerable. */
export interface CorpusSuggestion {
  /** The question as it will be shown: the template with real values substituted. */
  text: string;
  /**
   * The unfilled template. Optional in the pools (it is the same string as `text` there) and set
   * by `compose`, so a caller comparing placeholders across languages reads it, not `text`.
   */
  template?: string;
  needs: SuggestionNeed;
  /**
   * True when answering requires more than one document.
   *
   * WHY declared rather than inferred from wording: "Summarise the key points across these
   * documents" is nonsense on a one-document corpus, and a reader cannot tell that from the string.
   * Marking it here lets `compose` skip it until the corpus holds several documents, so the screen
   * never asks for a comparison the corpus cannot make — the original defect one level up.
   */
  needsSeveralDocuments?: boolean;
  /** i18n key for the text, so translation does not depend on the English string. */
  key: string;
}

/** The values a template may interpolate. A missing one makes the template unusable. */
export interface SuggestionVars {
  [name: string]: string | null | undefined;
  topic?: string | null;
  entity?: string | null;
  otherEntity?: string | null;
  relation?: string | null;
}

const PLACEHOLDER = /\{\{\s*(\w+)\s*\}\}/g;

/**
 * Fill a template, or refuse it.
 *
 * WHY refuse rather than substitute a default: the previous fallback was the literal string
 * `"the handbook"`, so a corpus of invoices was asked what *the handbook* said about refunds —
 * a fabricated subject presented as an authority. Returning `null` costs one suggestion and
 * removes the lie. `compose` then draws the next entry from that pool, so a slot is rarely lost.
 */
export function fillTemplate(template: string, vars: SuggestionVars): string | null {
  let missing = false;
  const filled = template.replace(PLACEHOLDER, (_whole, name: string) => {
    const value = vars[name];
    if (typeof value !== "string" || value.trim().length === 0) {
      missing = true;
      return "";
    }
    return value;
  });
  return missing ? null : filled;
}

/**
 * The suggestion pools for a workspace **without** a graph, one per file family.
 *
 * WHY every entry names a mechanism: "passage" for chunk retrieval, "page" for a PDF, "picture"
 * for a cross-modal hit. A reader learns what the workspace can do from the questions it is
 * offered, so a vague suggestion wastes the slot. Note that no entry invents a subject: the ones
 * that name something interpolate a real title and disappear when there is none.
 */
const FILE_POOLS: Record<CorpusCapability, CorpusSuggestion[]> = {
  text: [
    {
      text: "Which documents are in this workspace, and what is each about?",
      key: "query.corpusSuggest.text.inventory",
      needs: "text",
    },
    {
      text: "Find the passage that answers a specific question",
      key: "query.corpusSuggest.text.passage",
      needs: "text",
    },
    {
      text: "Summarise the key points across these documents",
      key: "query.corpusSuggest.text.summarise",
      needs: "text",
      needsSeveralDocuments: true,
    },
    {
      text: "Where is a particular term mentioned?",
      key: "query.corpusSuggest.text.term",
      needs: "text",
    },
    {
      // Single-document and title-free, so a one-document corpus with no usable title still fills
      // its four slots without being asked to compare anything.
      text: "What is this document about?",
      key: "query.corpusSuggest.text.about",
      needs: "text",
    },
    {
      text: "What does {{topic}} say about its main topic?",
      key: "query.corpusSuggest.text.topic",
      needs: "text",
    },
  ],
  pdf: [
    {
      text: "Which pages hold the tables and figures?",
      key: "query.corpusSuggest.pdf.figures",
      needs: "pdf",
    },
    {
      text: "Summarise the longest document",
      key: "query.corpusSuggest.pdf.summarise",
      needs: "pdf",
      needsSeveralDocuments: true,
    },
    {
      text: "Which pages of {{topic}} cover its main sections?",
      key: "query.corpusSuggest.pdf.page",
      needs: "pdf",
    },
    {
      text: "Quote the section of {{topic}} that sets out the rules",
      key: "query.corpusSuggest.pdf.quote",
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
      text: "What do the pictures in this workspace have in common?",
      key: "query.corpusSuggest.image.common",
      needs: "image",
      needsSeveralDocuments: true,
    },
    {
      // Demonstrates the affordance this kind of workspace paid for: attaching a picture to search.
      text: "Find pictures that look like one I attach",
      key: "query.corpusSuggest.image.attach",
      needs: "image",
    },
    {
      text: "How many pictures does this workspace hold?",
      key: "query.corpusSuggest.image.count",
      needs: "image",
    },
    {
      text: "Show me pictures similar to {{topic}}",
      key: "query.corpusSuggest.image.similar",
      needs: "image",
    },
  ],
};

/**
 * The pools for a workspace **with** a graph, keyed by retrieval mechanism rather than by file.
 *
 * WHY not keyed by file family like the normal pools: a graph answer is produced by an entity
 * lookup, a relationship walk, a community/theme report or a cross-document pass — the file the
 * entity came from is irrelevant. Naming the mechanism is what makes the four slots teach the
 * reader what a knowledge graph is for, and it is what the mode menu (`Focused`, `Broad`,
 * `Smart`) actually does.
 */
const GRAPH_POOLS = {
  entity: [
    {
      text: "What do we know about {{entity}}?",
      key: "query.corpusSuggest.graph.entity",
      needs: "graph",
    },
    {
      text: "Which entities appear most often in these documents?",
      key: "query.corpusSuggest.graph.topEntities",
      needs: "graph",
      needsSeveralDocuments: true,
    },
  ] as CorpusSuggestion[],
  relationship: [
    {
      // `relation` is the workspace's own declared relation type (SPEC-114), so this question can
      // only appear where the schema defines it — e.g. CITES on a research or legal workspace. It
      // leads the pool because it is the only question that demonstrates the operator's schema.
      text: "Which pairs of entities are connected by {{relation}}?",
      key: "query.corpusSuggest.graph.relation",
      needs: "graph",
    },
    {
      text: "How is {{entity}} connected to {{otherEntity}}?",
      key: "query.corpusSuggest.graph.connection",
      needs: "graph",
    },
    {
      text: "Which entity links the most other entities together?",
      key: "query.corpusSuggest.graph.hub",
      needs: "graph",
      needsSeveralDocuments: true,
    },
  ] as CorpusSuggestion[],
  theme: [
    {
      text: "What are the main themes across these documents?",
      key: "query.corpusSuggest.graph.themes",
      needs: "graph",
      needsSeveralDocuments: true,
    },
    {
      text: "Summarise the clusters in this knowledge graph",
      key: "query.corpusSuggest.graph.clusters",
      needs: "graph",
      needsSeveralDocuments: true,
    },
  ] as CorpusSuggestion[],
  document: [
    {
      text: "Which documents mention {{entity}}?",
      key: "query.corpusSuggest.graph.entityDocuments",
      needs: "graph",
    },
    {
      text: "Which themes appear in more than one document?",
      key: "query.corpusSuggest.graph.crossDocument",
      needs: "graph",
      needsSeveralDocuments: true,
    },
  ] as CorpusSuggestion[],
};

/** Which capabilities the corpus actually has, in the order a mixed corpus should show them. */
export function capabilitiesOf(shape: CorpusShape): CorpusCapability[] {
  const counts: Array<[CorpusCapability, number]> = [
    ["text", shape.textDocumentCount ?? 0],
    ["pdf", shape.pdfDocumentCount ?? 0],
    // A picture is only *askable* when the workspace embeds images; a text-only Normal workspace
    // cannot store one, so offering picture questions there would be the original bug again.
    ["image", shape.embedsImages === true ? (shape.imageDocumentCount ?? 0) : 0],
  ];
  return counts.filter(([, n]) => n > 0).map(([cap]) => cap);
}

/** Documents the family counts account for, ignoring `documentCount` (which may be unknown). */
function familyTotal(shape: CorpusShape): number {
  return (
    (shape.textDocumentCount ?? 0) +
    (shape.pdfDocumentCount ?? 0) +
    (shape.imageDocumentCount ?? 0)
  );
}

/**
 * How much the corpus can support.
 *
 * WHY an explicit zero and nothing else means "empty": the family counts come from one page of the
 * document list, so an in-flight request and a genuinely empty workspace both report zero
 * families. Only the server's `document_count` can tell them apart, and confusing the two would
 * flash "upload something" at a workspace that is merely loading.
 */
export function corpusStateOf(shape: CorpusShape): CorpusState {
  if (shape.documentCount === 0) return "empty";
  const known = shape.documentCount ?? familyTotal(shape);
  if (known <= 0) return "unknown";
  return known <= NASCENT_MAX ? "nascent" : "established";
}

/**
 * Draw up to `slots` suggestions, round-robin across pools, skipping any entry whose template
 * cannot be filled from real data.
 *
 * WHY round-robin: a corpus that is 90% photographs and 10% handbooks must still offer a handbook
 * question in its four slots, or its manual is unreachable from the empty screen. Ordering within a
 * pool is stable, so the list does not reshuffle between renders.
 *
 * Terminates because every pass either emits an entry or exhausts a pool.
 */
function compose(
  pools: CorpusSuggestion[][],
  vars: SuggestionVars,
  slots: number,
  state: CorpusState,
): CorpusSuggestion[] {
  const queues = pools.map((pool) => [...pool]);
  const out: CorpusSuggestion[] = [];
  let progressed = true;
  while (out.length < slots && progressed) {
    progressed = false;
    for (const queue of queues) {
      if (out.length >= slots) break;
      // Drain until one entry is printable: an unfillable template or a comparison the corpus is
      // too small to make is dropped, not defaulted.
      while (queue.length > 0) {
        const candidate = queue.shift()!;
        if (candidate.needsSeveralDocuments && state === "nascent") continue;
        const text = fillTemplate(candidate.text, vars);
        if (text === null) continue;
        out.push({ ...candidate, text, template: candidate.text });
        progressed = true;
        break;
      }
    }
  }
  return out;
}

/**
 * The topic word for a suggestion that names one.
 *
 * WHY exported and applied by the component too: `compose` substitutes the placeholder in the
 * English fallback, but the rendered string comes from `t(key, fallback)`, whose value also
 * contains the placeholder. Both call sites must use this one function or the screen can show a
 * raw placeholder — which is exactly what happened the first time.
 */
export function topicForCorpus(shape: CorpusShape): string | null {
  return topicFromTitles(shape.sampleTitles);
}

/**
 * A prose title from the corpus, or `null`.
 *
 * WHY not the first filename: a workspace of photographs would produce "What does 05-alpine-lake
 * say about refunds?", which is nonsense.
 *
 * WHY `null` and not a default: this used to return `"the handbook"`, which put a fabricated
 * authority in front of a corpus that has no handbook. A caller that cannot find a real title must
 * suppress the question, not invent one.
 */
export function topicFromTitles(titles: readonly string[] | undefined): string | null {
  const candidate = (titles ?? [])
    .map((t) => t.trim())
    .find((t) => t.length > 0 && /^[A-Za-z][A-Za-z0-9 _-]{2,}$/.test(t) && !/\d{2}-/.test(t));
  if (!candidate) return null;
  return candidate.replace(/\.[a-z0-9]+$/i, "").toLowerCase();
}

/**
 * The values every template may interpolate, derived from the corpus once.
 *
 * WHY one exported function: `compose` fills the English fallback, but the rendered string comes
 * from `t(key, fallback, vars)`, so the component must interpolate too. When those two sites
 * derived their own values, one of them showed a raw `CO-DESIGN_BENCH` slug while the other showed
 * `Co-Design Bench` — the same class of defect as the leaked `{{topic}}` this file already carries a
 * test for. Both callers now read this one function.
 */
export function suggestionVarsFor(shape: CorpusShape): SuggestionVars {
  const entities = (shape.sampleEntities ?? []).map(prettyEntity).filter((e) => e.length > 0);
  return {
    topic: topicFromTitles(shape.sampleTitles),
    entity: entities[0] ?? null,
    otherEntity: entities[1] ?? null,
    relation: shape.sampleRelations?.[0] ?? null,
  };
}

/** `SARAH_CHEN` → `Sarah Chen`, so a suggestion reads like a question and not like a slug. */
export function prettyEntity(raw: string): string {
  const words = raw.replace(/_/g, " ").trim().replace(/\s+/g, " ");
  if (!words) return "";
  // Lower the rest first: entity labels are upper-cased by the normaliser, so `CHEN` must become
  // `Chen` rather than staying `CHEN` under a first-letter-only transform. Hyphenated compounds are
  // capitalised per segment, so `CO-DESIGN` reads `Co-Design` and not `Co-design`.
  const capitalise = (segment: string) =>
    segment.charAt(0).toUpperCase() + segment.slice(1).toLowerCase();
  return words
    .split(" ")
    .map((word) => word.split("-").map(capitalise).join("-"))
    .join(" ");
}

/** The values a graph template may use, all taken from the corpus. */
function graphVars(shape: CorpusShape): SuggestionVars {
  return suggestionVarsFor(shape);
}

/** The values a file template may use. Normal workspaces have no entity graph to name. */
function fileVars(shape: CorpusShape): SuggestionVars {
  return { topic: topicFromTitles(shape.sampleTitles) };
}

/**
 * Suggestions for a workspace without a graph.
 *
 * When the corpus has not reported itself yet (`unknown`), the text pool is used: it holds two
 * questions that need no knowledge of the contents, and the "upload something" hint beside them
 * carries the real message. This is the pre-existing behaviour and it is still correct for the
 * loading case — the *empty* case is suppressed by the caller, which has `document_count`.
 */
export function suggestionsFor(
  shape: CorpusShape,
  slots: number = SUGGESTION_SLOTS,
): CorpusSuggestion[] {
  const state = corpusStateOf(shape);
  if (state === "empty") return [];
  const caps = capabilitiesOf(shape);
  const vars = fileVars(shape);
  if (caps.length === 0) return compose([FILE_POOLS.text], vars, slots, state);
  return compose(
    caps.map((cap) => FILE_POOLS[cap]),
    vars,
    slots,
    state,
  );
}

/**
 * Suggestions for a workspace with a graph.
 *
 * The pools are mechanisms, not files. Cross-document questions wait until the corpus has more
 * than a handful of documents, because "which themes appear in more than one document" cannot
 * resolve when there is one document — the same failure as the original bug, one level up.
 */
export function graphSuggestions(
  shape: CorpusShape,
  slots: number = SUGGESTION_SLOTS,
): CorpusSuggestion[] {
  const state = corpusStateOf(shape);
  if (state === "empty") return [];
  const pools = [GRAPH_POOLS.entity, GRAPH_POOLS.relationship, GRAPH_POOLS.theme];
  if (state === "established") pools.push(GRAPH_POOLS.document);
  return compose(pools, graphVars(shape), slots, state);
}

/** The one entry point: which composer a corpus earns. Never branches on the profile name. */
export function suggestionsForCorpus(
  shape: CorpusShape,
  slots: number = SUGGESTION_SLOTS,
): CorpusSuggestion[] {
  return shape.hasGraph ? graphSuggestions(shape, slots) : suggestionsFor(shape, slots);
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

const GRAPH_HEAD: CopyHead = {
  title: "Ask about your knowledge graph",
  description:
    "I can help you explore entities, find connections, and uncover insights from your documents.",
  subtitle: "Ask questions about your knowledge graph",
  keys: {
    title: "query.emptyTitle",
    description: "query.emptyDescription",
    suggestions: "query.suggestions",
    subtitle: "query.subtitle",
  },
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
    (shape.textDocumentCount ?? 0) + (shape.pdfDocumentCount ?? 0);
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
 *   2. A graph workspace gets a graph head **and graph suggestions** — it used to return here with
 *      a hardcoded list, which is why the default workspace kind never mentioned an entity it held.
 *   3. Everything else is a Normal workspace: the head names the kinds it actually holds, and the
 *      suggestions are drawn from the pools those kinds unlock.
 */
export function getQueryEmptyCopy(
  mode: QueryMode = "mix",
  corpus: CorpusShape | boolean = { hasGraph: true },
): QueryEmptyCopy {
  if (mode === "bypass") return CHAT_COPY;
  // Back-compat: the older call shape was `(mode, hasGraph)`.
  const shape: CorpusShape = typeof corpus === "boolean" ? { hasGraph: corpus } : corpus;
  const head = shape.hasGraph ? GRAPH_HEAD : headFor(shape);
  return {
    title: head.title,
    description: head.description,
    subtitle: head.subtitle,
    keys: head.keys,
    suggestions: suggestionsForCorpus(shape).map((s) => s.text),
  };
}

/** Suggestion keys, so the component can translate each one by its own key. */
export function getQueryEmptySuggestionKeys(
  mode: QueryMode,
  corpus: CorpusShape | boolean,
): string[] {
  if (mode === "bypass") return [];
  const shape: CorpusShape = typeof corpus === "boolean" ? { hasGraph: corpus } : corpus;
  return suggestionsForCorpus(shape).map((s) => s.key);
}

/**
 * Does this corpus have a question worth offering?
 *
 * WHY the component needs it separately: the "Try asking:" label must not sit above an empty grid.
 * A workspace with zero documents has no answerable question, so the honest screen is the upload
 * hint alone — not four cards that each produce a refusal on first click.
 */
export function hasSuggestions(mode: QueryMode, corpus: CorpusShape): boolean {
  if (mode === "bypass") return false;
  return suggestionsForCorpus(corpus).length > 0;
}

export function isChatQueryMode(mode: QueryMode): boolean {
  return mode === "bypass";
}

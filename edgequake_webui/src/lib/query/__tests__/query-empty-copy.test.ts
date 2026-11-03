import { describe, expect, it } from "vitest";
import {
  SUGGESTION_SLOTS,
  capabilitiesOf,
  corpusStateOf,
  fillTemplate,
  getQueryEmptyCopy,
  getQueryEmptySuggestionKeys,
  graphSuggestions,
  prettyEntity,
  suggestionVarsFor,
  suggestionsFor,
  suggestionsForCorpus,
  topicFromTitles,
  type CorpusShape,
} from "../query-empty-copy";
import {
  corpusShapeFromDocuments,
  documentFamily,
  isImageDocument,
} from "@/hooks/use-query-empty-corpus";

/** Twenty photographs: the corpus in the original bug report. */
const twentyPhotos: CorpusShape = {
  hasGraph: false,
  embedsImages: true,
  documentCount: 20,
  chunkCount: 20,
  imageDocumentCount: 20,
};

/** A normal workspace of markdown handbooks. */
const handbooks: CorpusShape = {
  hasGraph: false,
  embedsImages: false,
  documentCount: 12,
  chunkCount: 12,
  textDocumentCount: 12,
};

/** The common real case: a brand manual plus its product photographs. */
const mixed: CorpusShape = {
  hasGraph: false,
  embedsImages: true,
  documentCount: 30,
  chunkCount: 30,
  textDocumentCount: 8,
  pdfDocumentCount: 2,
  imageDocumentCount: 20,
};

/** A graph workspace with a real entity graph, as the default kind looks after ingest. */
const graphCorpus: CorpusShape = {
  hasGraph: true,
  documentCount: 24,
  chunkCount: 120,
  pdfDocumentCount: 24,
  sampleTitles: ["Annual Risk Report"],
  sampleEntities: ["Sarah_Chen", "Acme_Corp", "Berlin"],
  sampleRelations: ["WORKS_AT", "LOCATED_IN"],
};

describe("SPEC-183 capabilities of a corpus", () => {
  it("reports every family present, not just the majority", () => {
    expect(capabilitiesOf(mixed)).toEqual(["text", "pdf", "image"]);
    expect(capabilitiesOf(twentyPhotos)).toEqual(["image"]);
    expect(capabilitiesOf(handbooks)).toEqual(["text"]);
  });

  it('never unlocks picture questions on a workspace that cannot embed pictures', () => {
    // A text-only Normal workspace refuses an image upload (EC-180-18), so offering "find a
    // picture" there would promise something the workspace cannot do.
    const textOnlyWithImages: CorpusShape = {
      hasGraph: false,
      embedsImages: false,
      documentCount: 5,
      chunkCount: 5,
      imageDocumentCount: 5,
    };
    expect(capabilitiesOf(textOnlyWithImages)).toEqual([]);
    expect(suggestionsFor(textOnlyWithImages).map((s) => s.needs)).not.toContain("image");
  });
});

describe("SPEC-184 the five tests a default question must pass", () => {
  it("T1 answerable: an empty workspace is offered nothing", () => {
    const empty: CorpusShape = { hasGraph: false, embedsImages: true, documentCount: 0 };
    expect(corpusStateOf(empty)).toBe("empty");
    expect(suggestionsForCorpus(empty)).toEqual([]);
    // And the same for a graph workspace: no entities exist, so no entity question may appear.
    expect(
      graphSuggestions({ hasGraph: true, documentCount: 0 }),
    ).toEqual([]);
  });

  it("T1 answerable: an empty graph workspace names no entity it does not hold", () => {
    // Before the fix this returned four hardcoded sentences, none of which could mention an
    // entity — the default workspace kind was exempt from the corpus rule entirely.
    const noEntities: CorpusShape = { hasGraph: true, documentCount: 10, pdfDocumentCount: 10 };
    const texts = graphSuggestions(noEntities).map((s) => s.text).join(" ");
    expect(texts).not.toContain("{{");
    expect(texts).not.toMatch(/sarah|acme/i);
  });

  it("T1 answerable: a corpus with no known title is never asked about 'the handbook'", () => {
    // `topicFromTitles` used to return the literal string "the handbook", putting a fabricated
    // authority in front of a corpus that has none.
    expect(topicFromTitles([])).toBeNull();
    expect(topicFromTitles(undefined)).toBeNull();
    expect(topicFromTitles(["05-alpine-lake", "16-orange-poppies"])).toBeNull();
    for (const suggestion of suggestionsFor(twentyPhotos)) {
      expect(suggestion.text).not.toMatch(/handbook/i);
    }
  });

  it("T2 mechanism-true: four slots cover four mechanisms, not one repeated", () => {
    const keys = graphSuggestions(graphCorpus).map((s) => s.key);
    expect(keys).toHaveLength(SUGGESTION_SLOTS);
    // Each key's second-to-last segment is its pool; four distinct mechanisms in four slots.
    const mechanisms = keys.map((k) => k.split(".").at(-1));
    expect(new Set(mechanisms).size).toBe(keys.length);
  });

  it("T2 mechanism-true: a graph question is graph-shaped and a normal one is chunk-shaped", () => {
    expect(graphSuggestions(graphCorpus).map((s) => s.text).join(" ")).toMatch(/entit|connected|theme|graph/i);
    expect(suggestionsForCorpus(handbooks).map((s) => s.text).join(" ")).not.toMatch(/knowledge graph/i);
  });

  it("T3 vocabulary-true: a graph suggestion names a real entity, Title Cased", () => {
    const texts = graphSuggestions(graphCorpus).map((s) => s.text);
    expect(texts.join(" ")).toContain("Sarah Chen");
    // And a relation-shaped question only appears with a declared relation type.
    expect(texts.join(" ")).toContain("WORKS_AT");
    // Without declared relations that slot is dropped, never invented.
    const noRelations = graphSuggestions({ ...graphCorpus, sampleRelations: [] });
    expect(noRelations.map((s) => s.text).join(" ")).not.toContain("{{");
  });

  it("T4 distinct: no two of the four suggestions are the same string", () => {
    for (const shape of [twentyPhotos, handbooks, mixed, graphCorpus]) {
      const texts = suggestionsForCorpus(shape).map((s) => s.text);
      expect(new Set(texts).size, texts.join(" | ")).toBe(texts.length);
    }
  });

  it("T5 scope-safe: no suggestion carries an unresolved placeholder", () => {
    for (const shape of [twentyPhotos, handbooks, mixed, graphCorpus]) {
      for (const suggestion of suggestionsForCorpus(shape)) {
        expect(suggestion.text).not.toContain("{{");
        expect(suggestion.text).not.toContain("}}");
        expect(suggestion.text.trim()).not.toBe("");
      }
    }
  });
});

describe("SPEC-183 suggestions a classical RAG can actually answer", () => {
  it("offers picture questions on a picture corpus, never handbook questions", () => {
    const copy = getQueryEmptyCopy("naive", twentyPhotos);
    expect(copy.suggestions.join(" ")).not.toMatch(/refund|onboarding|deadlines/i);
    expect(copy.suggestions.join(" ")).toMatch(/picture/i);
    expect(copy.title).toMatch(/pictures/i);
  });

  it("offers both kinds on a mixed corpus — the case a majority vote loses", () => {
    // 20 pictures vs 10 text documents: a majority rule would print only picture questions and
    // the manual would be unreachable from the empty screen.
    const copy = getQueryEmptyCopy("naive", mixed);
    const joined = copy.suggestions.join(" ");
    expect(joined).toMatch(/picture/i);
    expect(joined).toMatch(/document|summarise|passage|page/i);
    expect(copy.suggestions).toHaveLength(SUGGESTION_SLOTS);
  });

  it("names both kinds in the mixed head sentence", () => {
    const copy = getQueryEmptyCopy("naive", mixed);
    expect(copy.title).toMatch(/documents and pictures/i);
    // And does not promise a page number for a picture.
    expect(copy.description).not.toMatch(/point to the page/i);
  });

  it("keeps a document-shaped greeting for a text corpus", () => {
    const copy = getQueryEmptyCopy("naive", handbooks);
    expect(copy.suggestions[0]).toMatch(/documents/i);
    expect(copy.keys.title).toBe("query.normalEmptyTitle");
  });

  it("every suggestion declares a capability the corpus has", () => {
    for (const shape of [twentyPhotos, handbooks, mixed]) {
      const caps = capabilitiesOf(shape);
      for (const suggestion of suggestionsFor(shape)) {
        expect(caps, `${suggestion.text} needs ${suggestion.needs}`).toContain(suggestion.needs);
      }
    }
  });

  it("interleaves so a minority kind still reaches the four slots", () => {
    const shape: CorpusShape = {
      hasGraph: false,
      embedsImages: true,
      documentCount: 100,
      chunkCount: 100,
      imageDocumentCount: 98,
      textDocumentCount: 2,
    };
    const kinds = suggestionsFor(shape).map((s) => s.needs);
    expect(kinds).toContain("text");
    expect(kinds).toContain("image");
  });

  it("fills all four slots when only one pool is available", () => {
    // Before this, `POOLS.text` held three entries and the grid rendered with a hole in it.
    expect(suggestionsFor(handbooks)).toHaveLength(SUGGESTION_SLOTS);
    expect(suggestionsFor(twentyPhotos)).toHaveLength(SUGGESTION_SLOTS);
  });

  it("a graph workspace and chat mode keep their own heads", () => {
    expect(getQueryEmptyCopy("mix", { hasGraph: true }).keys.title).toBe("query.emptyTitle");
    expect(getQueryEmptyCopy("bypass", mixed).keys.title).toBe("query.chatEmptyTitle");
  });

  it("exposes a distinct i18n key per suggestion", () => {
    const keys = getQueryEmptySuggestionKeys("naive", mixed);
    expect(keys).toHaveLength(SUGGESTION_SLOTS);
    expect(new Set(keys).size).toBe(keys.length);
    expect(keys.every((k) => k.startsWith("query.corpusSuggest."))).toBe(true);
  });
});

describe("SPEC-184 families the product can actually ingest", () => {
  it("attaches no questions to a family no ingest path can store", () => {
    // LAW-121-8: Office is a non-goal, so a DOCX row can only exist if the server refused it. A
    // pool for it would be an unanswerable promise, which is the defect this module exists to stop.
    expect(capabilitiesOf({ hasGraph: false, documentCount: 3, otherDocumentCount: 3 } as never)).toEqual(
      [],
    );
  });

  it("still offers text questions for a corpus of plain files", () => {
    // JSON and CSV are `text` on both sides (the server maps `text/csv` to DocKind::Text), so a
    // workspace of data files is answerable and must not be treated as unknown.
    const json: CorpusShape = {
      hasGraph: false,
      documentCount: 2,
      textDocumentCount: 2,
      sampleTitles: ["captions.json"],
    };
    expect(capabilitiesOf(json)).toEqual(["text"]);
    expect(suggestionsFor(json)).toHaveLength(SUGGESTION_SLOTS);
  });
});

describe("SPEC-184 corpus state", () => {
  it("distinguishes an empty workspace from a loading one", () => {
    expect(corpusStateOf({ hasGraph: false, documentCount: 0 })).toBe("empty");
    // No counts at all: loading, not empty. Falling back to the text pool here is correct,
    // because the alternative is flashing "upload something" at a workspace that has content.
    expect(corpusStateOf({ hasGraph: false })).toBe("unknown");
    expect(corpusStateOf({ hasGraph: false, textDocumentCount: 5 })).toBe("established");
  });

  it("holds cross-document questions back until there is more than one document", () => {
    const nascent: CorpusShape = { ...graphCorpus, documentCount: 2, chunkCount: 8 };
    expect(corpusStateOf(nascent)).toBe("nascent");
    const nascentKeys = graphSuggestions(nascent).map((s) => s.key);
    // The `document` pool is the cross-document one; none of it may appear on a two-doc corpus.
    expect(nascentKeys.some((k) => k.includes("Documents") || k.includes("crossDocument"))).toBe(
      false,
    );
    // With a real corpus the cross-document pool participates.
    const establishedPool = graphSuggestions(graphCorpus, 8).map((s) => s.key);
    expect(establishedPool.some((k) => k.includes("crossDocument"))).toBe(true);
  });

  it("never asks a one-document corpus to compare across documents", () => {
    // Measured before the fix: a single text document was greeted with "Summarise the key points
    // across these documents" — plural, on a corpus of one. The entry declares
    // `needsSeveralDocuments`, and compose skips it until the corpus is `established`.
    const single: CorpusShape = {
      hasGraph: false,
      documentCount: 1,
      textDocumentCount: 1,
      sampleTitles: ["policy.md"],
    };
    expect(corpusStateOf(single)).toBe("nascent");
    const texts = suggestionsFor(single).map((s) => s.text).join(" ");
    expect(texts).not.toMatch(/across these documents|these documents/i);
    // And the same for a picture corpus: "what do the pictures have in common" needs more than one.
    const onePicture: CorpusShape = {
      hasGraph: false,
      embedsImages: true,
      documentCount: 1,
      imageDocumentCount: 1,
    };
    expect(suggestionsFor(onePicture).map((s) => s.text).join(" ")).not.toMatch(/in common/i);
  });

  it("lets the cross-document questions return once the corpus can answer them", () => {
    const texts = suggestionsFor(handbooks).map((s) => s.text).join(" ");
    expect(texts).toMatch(/across these documents/i);
  });
});

describe("SPEC-184 template filling", () => {
  it("refuses a template rather than inventing a value", () => {
    expect(fillTemplate("What does {{topic}} say?", { topic: "risk report" })).toBe(
      "What does risk report say?",
    );
    expect(fillTemplate("What does {{topic}} say?", { topic: null })).toBeNull();
    expect(fillTemplate("What does {{topic}} say?", {})).toBeNull();
    expect(fillTemplate("What does {{topic}} say?", { topic: "   " })).toBeNull();
  });

  it("renders a real slug as prose", () => {
    expect(prettyEntity("SARAH_CHEN")).toBe("Sarah Chen");
    expect(prettyEntity("acme_corp")).toBe("Acme Corp");
    expect(prettyEntity("  ")).toBe("");
  });

  it("gives the component and the lib the same values, so neither shows a raw slug", () => {
    // The rendered string comes from `t(key, fallback, vars)`, so the component interpolates too.
    // When the two sites derived their own values, the screen showed `CO-DESIGN_BENCH` while the
    // lib's own output said `Co-Design Bench` — the same class of bug as a leaked `{{topic}}`.
    const shape: CorpusShape = {
      hasGraph: true,
      documentCount: 12,
      sampleEntities: ["CO-DESIGN_BENCH", "WEBCOMPASS-EDIT"],
      sampleRelations: ["RELATED_TO"],
    };
    const vars = suggestionVarsFor(shape);
    expect(vars.entity).toBe("Co-Design Bench");
    // The hyphen is part of the label, so it is preserved, not silently turned into a space.
    expect(vars.otherEntity).toBe("Webcompass-Edit");
    // Every suggestion the lib produces must be reproducible from those same vars.
    for (const suggestion of graphSuggestions(shape)) {
      expect(fillTemplate(suggestion.template ?? suggestion.text, vars)).toBe(suggestion.text);
    }
  });

  it("never leaves a slug in a rendered suggestion", () => {
    const shape: CorpusShape = {
      hasGraph: true,
      documentCount: 12,
      sampleEntities: ["CO-DESIGN_BENCH", "WEBCOMPASS-EDIT"],
    };
    for (const suggestion of graphSuggestions(shape)) {
      expect(suggestion.text).not.toMatch(/[A-Z]{2,}_[A-Z]/);
    }
  });
});

describe("SPEC-183 document families", () => {
  it("classifies by source_type first, then MIME, then extension", () => {
    expect(documentFamily({ source_type: "image" })).toBe("image");
    expect(documentFamily({ content_type: "application/pdf" })).toBe("pdf");
    expect(documentFamily({ content_type: "text/markdown" })).toBe("text");
    // Office is a v1 non-goal (LAW-121-8): nothing ingests it, so it is not a family with
    // questions — the row falls through to `other`, which carries no pool.
    expect(documentFamily({ file_name: "report.docx" })).toBe("other");
    expect(documentFamily({ file_name: "budget.xlsx" })).toBe("other");
    expect(documentFamily({ file_name: "notes.md" })).toBe("text");
    expect(documentFamily({ file_name: "photo.JPG" })).toBe("image");
    expect(documentFamily({})).toBe("other");
  });

  it("keeps the boolean helper used elsewhere", () => {
    expect(isImageDocument({ source_type: "image" })).toBe(true);
    expect(isImageDocument({ content_type: "text/plain" })).toBe(false);
  });

  it("counts every family from a real page of the documents API", () => {
    const rows = [
      { title: "20-beagle-portrait", source_type: "image" },
      { title: "05-alpine-lake", content_type: "image/jpeg" },
      { title: "policy.md", content_type: "text/markdown" },
      { title: "contract.pdf", content_type: "application/pdf" },
      { title: "budget.xlsx" },
    ];
    const shape = corpusShapeFromDocuments(false, true, rows);
    expect(shape).toMatchObject({
      imageDocumentCount: 2,
      textDocumentCount: 1,
      pdfDocumentCount: 1,
      chunkCount: 5,
    });
    expect(shape.sampleTitles).toContain("20-beagle-portrait");
  });

  it("carries the server document count so an empty workspace is distinguishable", () => {
    const shape = corpusShapeFromDocuments(false, false, [], 0);
    expect(shape.documentCount).toBe(0);
    expect(corpusStateOf(shape)).toBe("empty");
  });
});

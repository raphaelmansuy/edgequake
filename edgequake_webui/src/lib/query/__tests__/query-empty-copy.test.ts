import { describe, expect, it } from "vitest";
import {
  SUGGESTION_SLOTS,
  capabilitiesOf,
  getQueryEmptyCopy,
  getQueryEmptySuggestionKeys,
  suggestionsFor,
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
  chunkCount: 20,
  imageDocumentCount: 20,
};

/** A normal workspace of markdown handbooks. */
const handbooks: CorpusShape = {
  hasGraph: false,
  embedsImages: false,
  chunkCount: 12,
  textDocumentCount: 12,
};

/** The common real case: a brand manual plus its product photographs. */
const mixed: CorpusShape = {
  hasGraph: false,
  embedsImages: true,
  chunkCount: 30,
  textDocumentCount: 8,
  pdfDocumentCount: 2,
  imageDocumentCount: 20,
};

describe("SPEC-183 capabilities of a corpus", () => {
  it('reports every family present, not just the majority', () => {
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
      chunkCount: 5,
      imageDocumentCount: 5,
    };
    expect(capabilitiesOf(textOnlyWithImages)).toEqual([]);
    expect(suggestionsFor(textOnlyWithImages).map((s) => s.needs)).not.toContain("image");
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
    expect(joined).toMatch(/refund|deadlines|onboarding|passage/i);
    expect(copy.suggestions).toHaveLength(SUGGESTION_SLOTS);
  });

  it("names both kinds in the mixed head sentence", () => {
    const copy = getQueryEmptyCopy("naive", mixed);
    expect(copy.title).toMatch(/documents and pictures/i);
    // And does not promise a page number for a picture.
    expect(copy.description).not.toMatch(/point to the page/i);
  });

  it("keeps the text questions for a text corpus", () => {
    const copy = getQueryEmptyCopy("naive", handbooks);
    expect(copy.suggestions[0]).toMatch(/refund/i);
    expect(copy.keys.title).toBe("query.normalEmptyTitle");
  });

  it("every suggestion declares the capability it needs, and only present ones are used", () => {
    for (const shape of [twentyPhotos, handbooks, mixed]) {
      const caps = capabilitiesOf(shape);
      for (const suggestion of suggestionsFor(shape)) {
        expect(caps, `${suggestion.text} needs ${suggestion.needs}`).toContain(suggestion.needs);
        // No unresolved template may reach the screen.
        expect(suggestion.text).not.toContain("{{");
      }
    }
  });

  it("interleaves so a minority kind still reaches the four slots", () => {
    const shape: CorpusShape = {
      hasGraph: false,
      embedsImages: true,
      chunkCount: 100,
      imageDocumentCount: 98,
      textDocumentCount: 2,
    };
    const kinds = suggestionsFor(shape).map((s) => s.needs);
    expect(kinds).toContain("text");
    expect(kinds).toContain("image");
  });

  it("falls back to the text pool when the corpus has said nothing yet", () => {
    const empty: CorpusShape = { hasGraph: false, embedsImages: true, chunkCount: 0 };
    expect(suggestionsFor(empty).length).toBeGreaterThan(0);
    expect(capabilitiesOf(empty)).toEqual([]);
  });

  it("a graph workspace and chat mode are untouched by corpus logic", () => {
    expect(getQueryEmptyCopy("mix", { hasGraph: true }).keys.title).toBe("query.emptyTitle");
    expect(getQueryEmptyCopy("bypass", mixed).keys.title).toBe("query.chatEmptyTitle");
  });

  it("leaves no placeholder for a translated string to leak", () => {
    // The bug this pins: `suggestionsFor` substituted `{{topic}}` in the English fallback, but the
    // component renders `t(key, fallback, opts)`, whose *translated* value also contains the
    // placeholder. Both sides looked right in isolation and the screen showed "{{topic}}".
    // Every suggestion that names a topic must therefore be renderable from data alone.
    const shape: CorpusShape = {
      hasGraph: false,
      embedsImages: false,
      chunkCount: 3,
      textDocumentCount: 3,
      sampleTitles: ["Brand Handbook"],
    };
    for (const suggestion of suggestionsFor(shape)) {
      expect(suggestion.text).not.toContain("{{");
      expect(suggestion.text).not.toContain("}}");
    }
    // And the topic the component substitutes is the same value the lib used.
    expect(topicFromTitles(shape.sampleTitles)).toBe("brand handbook");
  });

  it("exposes a distinct i18n key per suggestion", () => {
    const keys = getQueryEmptySuggestionKeys("naive", mixed);
    expect(keys).toHaveLength(SUGGESTION_SLOTS);
    expect(new Set(keys).size).toBe(keys.length);
    expect(keys.every((k) => k.startsWith("query.corpusSuggest."))).toBe(true);
  });
});

describe("SPEC-183 topic naming", () => {
  it("uses a prose title, not a numbered photograph", () => {
    // "What does 05-alpine-lake say about refunds?" is nonsense.
    expect(topicFromTitles(["05-alpine-lake", "Brand Handbook"])).toBe("brand handbook");
    expect(topicFromTitles(["01-forest-path", "16-orange-poppies"])).toBe("the handbook");
    expect(topicFromTitles([])).toBe("the handbook");
    expect(topicFromTitles(undefined)).toBe("the handbook");
  });
});

describe("SPEC-183 document families", () => {
  it("classifies by source_type first, then MIME, then extension", () => {
    expect(documentFamily({ source_type: "image" })).toBe("image");
    expect(documentFamily({ content_type: "application/pdf" })).toBe("pdf");
    expect(documentFamily({ content_type: "text/markdown" })).toBe("text");
    expect(documentFamily({ file_name: "report.docx" })).toBe("office");
    expect(documentFamily({ file_name: "budget.xlsx" })).toBe("office");
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
      officeDocumentCount: 1,
      chunkCount: 5,
    });
    expect(shape.sampleTitles).toContain("20-beagle-portrait");
  });
});

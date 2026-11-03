import { describe, expect, it } from "vitest";

import en from "@/locales/en.json";
import fr from "@/locales/fr.json";
import zh from "@/locales/zh.json";
import {
  graphSuggestions,
  suggestionsFor,
  type CorpusSuggestion,
} from "@/lib/query/query-empty-copy";

type Tree = { [key: string]: string | Tree };
type Locale = { query: Tree };

const locales = { en, fr, zh } as unknown as Record<string, Locale>;

function get(tree: Tree, path: string): string | undefined {
  // Suggestion keys are absolute (`query.corpusSuggest.…`) and the tree starts at `query`.
  const parts = path.replace(/^query\./, "").split(".");
  let node: string | Tree | undefined = tree;
  for (const part of parts) {
    if (typeof node !== "object" || node === undefined) return undefined;
    node = node[part];
  }
  return typeof node === "string" ? node : undefined;
}

const placeholders = (text: string) =>
  [...text.matchAll(/\{\{(\w+)\}\}/g)].map((m) => m[1]!).sort();

/** Every pool, reached by giving each composer a corpus that unlocks all of it. */
const allSuggestions: CorpusSuggestion[] = [
  ...suggestionsFor(
    {
      hasGraph: false,
      embedsImages: true,
      documentCount: 40,
      textDocumentCount: 10,
      pdfDocumentCount: 10,
      officeDocumentCount: 10,
      imageDocumentCount: 10,
      sampleTitles: ["Brand Handbook"],
    },
    32,
  ),
  ...graphSuggestions(
    {
      hasGraph: true,
      documentCount: 40,
      sampleTitles: ["Annual Report"],
      sampleEntities: ["Sarah_Chen", "Acme_Corp"],
      sampleRelations: ["WORKS_AT"],
    },
    32,
  ),
];

describe("SPEC-184 suggestion i18n parity", () => {
  it("reaches every pool, so the parity check below is not vacuous", () => {
    const families = new Set(allSuggestions.map((s) => s.key.split(".").slice(2, 3)[0]));
    expect([...families].sort()).toEqual(["graph", "image", "pdf", "text"]);
  });

  it("defines every suggestion key in en, fr and zh", () => {
    for (const [name, locale] of Object.entries(locales)) {
      for (const suggestion of allSuggestions) {
        const value = get(locale.query, suggestion.key);
        expect(value, `${name}: ${suggestion.key} is missing`).toBeTypeOf("string");
        expect(value?.trim(), `${name}: ${suggestion.key} is empty`).not.toBe("");
      }
    }
  });

  it("keeps the same {{placeholders}} in every language as the code's template", () => {
    for (const [name, locale] of Object.entries(locales)) {
      for (const suggestion of allSuggestions) {
        expect(
          placeholders(get(locale.query, suggestion.key) ?? ""),
          `${name}: ${suggestion.key}`,
        ).toEqual(placeholders(suggestion.template));
      }
    }
  });

  it("has no suggestion key in a locale that the code never asks for", () => {
    const asked = new Set(allSuggestions.map((s) => s.key.replace(/^query\./, "")));
    const defined = new Set<string>();
    const walk = (tree: Tree, prefix: string) => {
      for (const [key, value] of Object.entries(tree)) {
        const path = `${prefix}${key}`;
        if (typeof value === "string") defined.add(path);
        else walk(value, `${path}.`);
      }
    };
    walk(locales.en!.query.corpusSuggest as Tree, "corpusSuggest.");
    const orphans = [...defined].filter((key) => !asked.has(key));
    expect(orphans).toEqual([]);
  });
});

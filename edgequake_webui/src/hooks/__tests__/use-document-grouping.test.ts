import { describe, expect, it } from "vitest";
import { groupDocumentsByTopic } from "../use-document-grouping";
import type { Document } from "@/types";

function makeDoc(id: string, topic?: string): Document {
  return { id, enrichment_topic: topic } as Document;
}

describe("groupDocumentsByTopic", () => {
  it("groups documents by topic", () => {
    const docs = [
      makeDoc("1", "Psikologi"),
      makeDoc("2", "Politik"),
      makeDoc("3", "Psikologi"),
    ];
    const result = groupDocumentsByTopic(docs);
    expect(result.get("Psikologi")).toHaveLength(2);
    expect(result.get("Politik")).toHaveLength(1);
  });

  it("puts docs without topic into 'Tanpa Topic'", () => {
    const docs = [makeDoc("1", undefined), makeDoc("2", ""), makeDoc("3", "Politik")];
    const result = groupDocumentsByTopic(docs);
    expect(result.get("Tanpa Topic")).toHaveLength(2);
    expect(result.get("Politik")).toHaveLength(1);
  });

  it("'Tanpa Topic' is always last key in the map", () => {
    const docs = [
      makeDoc("1", undefined),
      makeDoc("2", "Zzz Topic"),
      makeDoc("3", "Aaa Topic"),
    ];
    const result = groupDocumentsByTopic(docs);
    const keys = Array.from(result.keys());
    expect(keys[keys.length - 1]).toBe("Tanpa Topic");
  });

  it("sorts other topics alphabetically", () => {
    const docs = [
      makeDoc("1", "Zebra"),
      makeDoc("2", "Apple"),
      makeDoc("3", "Mango"),
    ];
    const result = groupDocumentsByTopic(docs);
    const keys = Array.from(result.keys());
    expect(keys).toEqual(["Apple", "Mango", "Zebra"]);
  });

  it("returns empty map for empty input", () => {
    expect(groupDocumentsByTopic([]).size).toBe(0);
  });

  it("returns single group when all docs have same topic", () => {
    const docs = [makeDoc("1", "ML"), makeDoc("2", "ML")];
    const result = groupDocumentsByTopic(docs);
    expect(result.size).toBe(1);
    expect(result.get("ML")).toHaveLength(2);
  });
});

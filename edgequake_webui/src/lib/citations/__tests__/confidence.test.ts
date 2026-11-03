import { describe, expect, it } from "vitest";
import type { QueryContext } from "@/types";
import {
  calculateAnswerConfidence,
  calculateConfidence,
  getConfidenceLabel,
  isRefusalAnswer,
} from "../confidence";

/** A context shaped like the one that produced the bug: 20 image chunks, no text. */
function twentyImageChunks(): QueryContext {
  return {
    chunks: Array.from({ length: 20 }, (_, i) => ({
      id: `c${i}`,
      content: "",
      // Real fused range measured on a Normal workspace (api/RESULTS.md).
      score: 1 / (61 + i),
      document_id: `d${i}`,
      file_path: `0${i}-photo`,
      source_modality: "image" as const,
    })),
    entities: [],
    relationships: [],
  } as unknown as QueryContext;
}

/** The exact sentence the model produced on that workspace. */
const REFUSAL =
  "The provided context consists solely of image placeholders without any textual " +
  "information, descriptions, or data concerning nature. Consequently, there is " +
  "insufficient evidence available in the knowledge base to address your query.";

describe("SPEC-183 answer-aware confidence", () => {
  it("does not label a refusal Strong, however much evidence came back", () => {
    const context = twentyImageChunks();
    // The evidence breadth alone still reads "Strong" — that is the whole problem.
    expect(getConfidenceLabel(calculateConfidence(context) ?? 0).defaultLabel).toBe("Strong");

    // With the answer read, there is no confidence to assert.
    expect(calculateAnswerConfidence(context, REFUSAL)).toBeNull();
  });

  it("still reports confidence when the answer actually answers", () => {
    const context = twentyImageChunks();
    const answer = "The refund clause is on page 4 of the handbook [1].";
    const score = calculateAnswerConfidence(context, answer);
    expect(score).not.toBeNull();
    expect(getConfidenceLabel(score ?? 0).defaultLabel).toBe("Strong");
  });

  it("recognises the shapes a model uses to decline", () => {
    for (const refusal of [
      REFUSAL,
      "Not answerable.",
      "I cannot answer that from the provided context.",
      "The documents do not contain information about refunds.",
      "There is no relevant textual content in these sources.",
      "I don't have enough information to answer.",
      "The context is entirely image placeholders.",
      "Unable to determine the answer.",
      // The exact sentence the live app produced on the 20-picture workspace, passive voice.
      "Based on the handbook content provided in the collection, what the handbook states regarding refunds cannot be determined.",
      "This cannot be determined from the sources.",
    ]) {
      expect(isRefusalAnswer(refusal), refusal).toBe(true);
    }
  });

  it("does not mistake an answer that merely mentions evidence for a refusal", () => {
    for (const answer of [
      "The evidence shows refunds are accepted within 30 days [1].",
      "Insufficient sleep was reported by 42% of respondents [2].",
      "The handbook covers onboarding in chapter 3 [1].",
      "I can answer this: the clause is on page 4 [1].",
      "",
      null,
      undefined,
    ]) {
      expect(isRefusalAnswer(answer), String(answer)).toBe(false);
    }
  });

  it("treats an empty context as nothing to judge, refusal or not", () => {
    const empty = { chunks: [], entities: [], relationships: [] } as unknown as QueryContext;
    expect(calculateAnswerConfidence(empty, "Anything at all.")).toBeNull();
  });
});

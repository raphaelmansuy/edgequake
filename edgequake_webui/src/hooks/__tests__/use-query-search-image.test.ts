import { describe, expect, it } from "vitest";
import { searchImageMessages } from "../use-query-search-image";

describe("searchImageMessages", () => {
  const FALLBACK = "Find matches for this image";

  it("keeps the user's words as the query", () => {
    expect(searchImageMessages("a red bicycle", false, FALLBACK)).toEqual({
      message: "a red bicycle",
      display: "a red bicycle",
    });
    // With a picture too: the words still count, and are shown.
    expect(searchImageMessages("a red bicycle", true, FALLBACK)).toEqual({
      message: "a red bicycle",
      display: "a red bicycle",
    });
  });

  it("sends no text for a picture-only search but still shows a turn", () => {
    // The regression: the label was sent as the text query. On a normal workspace that
    // text is embedded as a competing vector, and the pizza probe stopped returning the
    // pizza. The label belongs in `display` only.
    expect(searchImageMessages("", true, FALLBACK)).toEqual({
      message: "",
      display: FALLBACK,
    });
    expect(searchImageMessages("   ", true, FALLBACK)).toEqual({
      message: "",
      display: FALLBACK,
    });
  });

  it("has nothing to send without words or a picture", () => {
    expect(searchImageMessages("", false, FALLBACK)).toEqual({ message: "", display: "" });
  });
});

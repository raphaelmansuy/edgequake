import { describe, expect, it } from "bun:test";
import { SCROLL_CONTAINED_LIST_CLASS } from "@/hooks/use-scroll-contained-wheel";

describe("use-scroll-contained-wheel", () => {
  it("exports overscroll containment class for popover lists", () => {
    expect(SCROLL_CONTAINED_LIST_CLASS).toContain("overscroll-contain");
  });
});

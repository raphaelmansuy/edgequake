import { describe, expect, it } from "vitest";
import { renderKatexToHtml } from "../utils/katex-render";

describe("renderKatexToHtml", () => {
  it("produces inline KaTeX HTML", () => {
    const html = renderKatexToHtml("E=mc^2", false);
    expect(html).toBeTruthy();
    expect(html).toContain('class="katex"');
    expect(html).not.toContain("katex-display");
  });

  it("produces display-mode KaTeX HTML", () => {
    const html = renderKatexToHtml("\\int_0^1 x^2 \\, dx", true);
    expect(html).toBeTruthy();
    expect(html).toContain("katex-display");
  });
});

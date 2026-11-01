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

  // Security regression gate: math arrives from document chunks, model answers and pasted
  // queries, so it is untrusted input. With `trust: true` KaTeX honours \href and emits a live
  // `javascript:` link (verified against the pinned KaTeX); this test fails if that returns.
  it("never emits a javascript: href from untrusted math", () => {
    const html = renderKatexToHtml(
      "\\href{javascript:alert(document.cookie)}{click}",
      false,
    );
    expect(html ?? "").not.toMatch(/href="javascript:/i);
  });

  it("never emits an image URL from untrusted math", () => {
    const html = renderKatexToHtml(
      "\\includegraphics{https://attacker.example/pixel.png}",
      false,
    );
    expect(html ?? "").not.toMatch(/src="https:\/\/attacker\.example/i);
  });
});

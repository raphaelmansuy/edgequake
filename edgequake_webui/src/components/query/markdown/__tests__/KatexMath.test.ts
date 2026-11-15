import katex from "katex";
import { JSDOM } from "jsdom";
import { describe, expect, it } from "vitest";
import { renderKatexToHtml } from "../utils/katex-render";
import { displayLatexFromEquationHtml } from "../utils/math-token";
import { sanitizeKatexHtml } from "../utils/sanitize-katex-html";

function documentFor(html: string): Document {
  return new JSDOM(`<!DOCTYPE html><body>${html}</body>`).window.document;
}

/** The markup must not be able to navigate, load a URL, or run an event handler. */
function expectInert(html: string): void {
  const doc = documentFor(html);
  expect(
    doc.querySelector("a, img, script, iframe, object, embed, foreignObject, math, link, form"),
  ).toBeNull();
  for (const el of doc.body.querySelectorAll("*")) {
    for (const name of el.getAttributeNames()) {
      expect(name.toLowerCase().startsWith("on")).toBe(false);
      expect(["href", "src", "xlink:href", "action", "formaction"]).not.toContain(name);
      const style = el.getAttribute("style") ?? "";
      expect(style).not.toMatch(/url\s*\(|javascript\s*:|expression\s*\(/i);
    }
  }
}

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

  // Issue #536: `<equation>\href{javascript:alert(1)}{click me}</equation>` used to
  // skip the HTML sanitizer and, with KaTeX `trust: true`, become a live anchor.
  it("renders an equation body without a javascript anchor", () => {
    const latex = displayLatexFromEquationHtml(
      "<equation>\\href{javascript:alert(1)}{click me}</equation>",
    );
    expect(latex).toBe("\\href{javascript:alert(1)}{click me}");
    const html = renderKatexToHtml(latex ?? "", true);
    expect(html).toBeTruthy();
    expectInert(html ?? "");
    expect(html).toContain('class="katex"');
  });

  it("keeps radical glyphs and layout styles", () => {
    const html = renderKatexToHtml("\\sqrt{\\frac{a}{b}}", false) ?? "";
    expect(html).toContain("<svg");
    expect(html).toContain("style=");
    const path = documentFor(html).querySelector("svg path");
    expect(path?.getAttribute("d")?.length ?? 0).toBeGreaterThan(10);
  });

  it("strips a javascript anchor even when KaTeX was trusted", () => {
    const dirty = katex.renderToString("\\href{javascript:alert(1)}{click me}", {
      throwOnError: false,
      strict: false,
      trust: true,
      output: "html",
    });
    expect(dirty).toMatch(/href="javascript:alert\(1\)"/);
    const clean = sanitizeKatexHtml(dirty);
    expectInert(clean);
    expect(clean).toContain('class="katex"');
    expect(documentFor(clean).body.textContent?.replace(/\s/g, "")).toContain("clickme");
  });

  it("strips trusted includegraphics and htmlStyle urls", () => {
    const image = katex.renderToString(
      "\\includegraphics{https://attacker.example/pixel.png}",
      { throwOnError: false, trust: true, output: "html" },
    );
    expect(image).toContain("<img");
    expectInert(sanitizeKatexHtml(image));

    const styled = katex.renderToString(
      "\\htmlStyle{background:url(javascript:alert(1))}{x}",
      { throwOnError: false, trust: true, output: "html" },
    );
    expect(styled).toMatch(/url\(javascript:alert\(1\)\)/);
    const clean = sanitizeKatexHtml(styled);
    expectInert(clean);
    expect(clean).not.toMatch(/url\s*\(/i);
  });

  it("does not turn a quoted htmlClass into an event handler", () => {
    const dirty = katex.renderToString(
      '\\htmlClass{" onclick=alert(1) x=}{x}',
      { throwOnError: false, trust: true, output: "html" },
    );
    expectInert(sanitizeKatexHtml(dirty));
  });
});

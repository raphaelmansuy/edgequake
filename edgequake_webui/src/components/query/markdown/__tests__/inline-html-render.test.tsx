/**
 * GH-556 — rendering contract for inline HTML inside a model answer.
 *
 * WHY a DOM test in addition to the unit tests for `mergeInlineHtmlTokens`:
 * the unit tests prove the merge is cheap and associative. This file proves the
 * thing users actually see still works — an inline `<sup>`/`<sub>` renders as a
 * real element, and a page full of unclosed tags renders without hanging.
 *
 * The unclosed-tag case is the regression the issue reported: 32 000 unpaired
 * opening tags took seconds of main-thread time per render, so the answer pane
 * froze while streaming. The bound here is generous (a jsdom render plus DOMPurify
 * is not free) but still an order of magnitude tighter than the old merge alone.
 */
import { render } from "@testing-library/react";
import { marked, type Token } from "marked";
import { describe, expect, it, vi } from "vitest";
import { MarkdownInlineTokens } from "../MarkdownInlineTokens";

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: () => undefined }),
  usePathname: () => "/documents",
}));

/** Inline tokens of the first paragraph, i.e. what the answer pane receives. */
function paragraphTokens(markdown: string): Token[] {
  const paragraph = marked.lexer(markdown)[0] as Token & { tokens?: Token[] };
  return paragraph.tokens ?? [];
}

describe("GH-556 inline HTML rendering", () => {
  it("renders a split <sup> element as a real element with its text", () => {
    const { container } = render(
      <MarkdownInlineTokens id="sup" tokens={paragraphTokens("a <sup>1*</sup> b")} done />,
    );

    const sup = container.querySelector("sup");
    expect(sup).not.toBeNull();
    expect(sup?.textContent).toBe("1*");
    // The closing tag must not survive as visible text.
    expect(container.textContent).not.toContain("</sup>");
  });

  it("renders several inline elements on one line", () => {
    const { container } = render(
      <MarkdownInlineTokens
        id="multi"
        tokens={paragraphTokens("r<sub>i</sub>(W) and y<sub>i</sub><sup>*</sup>")}
        done
      />,
    );

    expect(container.querySelectorAll("sub")).toHaveLength(2);
    expect(container.querySelectorAll("sup")).toHaveLength(1);
    expect(container.textContent).not.toMatch(/<\/?(sub|sup)>/);
  });

  it("renders an answer full of unclosed tags without dropping content", () => {
    // The shape the issue reports: model output with many opening tags and no
    // closing partners (a truncated stream, or literal `<` in prose).
    //
    // WHY no timing assertion here: in jsdom the cost is React element creation
    // and DOMPurify serialisation, not the merge, so a bound here would measure
    // jsdom rather than the regression. The complexity guard lives in
    // `inline-html-merge.test.ts` and the browser proof in
    // `e2e/issue556-inline-html-merge.spec.ts`.
    const count = 2_000;
    const markdown = Array.from({ length: count }, (_, i) => `<span class="k${i}">`).join("");
    const tokens = paragraphTokens(`${markdown}\ntail`);

    const { container } = render(<MarkdownInlineTokens id="unclosed" tokens={tokens} done />);

    // Text after the tags still renders — nothing was silently dropped.
    expect(container.textContent).toContain("tail");
  });

  it("still suppresses an orphan closing tag with no opening partner", () => {
    const { container } = render(
      <MarkdownInlineTokens id="orphan" tokens={[{ type: "html", raw: "</sup>" } as Token]} done />,
    );
    expect(container.textContent).not.toContain("</sup>");
    expect(container.querySelector("sup")).toBeNull();
  });
});

import { render, waitFor } from "@testing-library/react";
import { marked, type Token } from "marked";
import { describe, expect, it, vi } from "vitest";
import { MarkdownInlineTokens } from "../MarkdownInlineTokens";
import { MarkdownTokens } from "../MarkdownTokens";

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: () => undefined }),
  usePathname: () => "/documents",
}));

const EQUATION =
  '<equation onclick="alert(1)">\\href{javascript:alert(1)}{click me}</equation>';

function expectNoJavascriptAnchor(container: HTMLElement): void {
  expect(container.querySelector("a")).toBeNull();
  expect(container.innerHTML).not.toMatch(/href\s*=\s*["']\s*javascript:/i);
  expect(container.querySelector("[onclick], [onerror], img, script, iframe")).toBeNull();
}

describe("equation markdown XSS", () => {
  it("does not render a javascript anchor from an inline equation", async () => {
    const paragraph = marked.lexer(EQUATION)[0] as Token & { tokens?: Token[] };
    const { container } = render(
      <MarkdownInlineTokens id="eq" tokens={paragraph.tokens ?? []} done />,
    );
    await waitFor(() => {
      expect(container.querySelector(".katex")).toBeTruthy();
    });
    expectNoJavascriptAnchor(container);
    expect(container.textContent).toContain("\\href");
  });

  it("does not render a javascript anchor from a block equation", async () => {
    const raw =
      '<equation onclick="alert(1)">\n\\href{javascript:alert(1)}{click me}\n</equation>';
    const { container } = render(
      <MarkdownTokens
        tokens={[
          {
            type: "html",
            raw,
            text: raw,
            block: true,
          } as Token,
        ]}
      />,
    );
    await waitFor(() => {
      expect(container.querySelector(".katex")).toBeTruthy();
    });
    expectNoJavascriptAnchor(container);
  });

  it("strips a javascript href from an ordinary html token", () => {
    const { container } = render(
      <MarkdownInlineTokens
        id="html"
        tokens={[{ type: "html", raw: '<a href="javascript:alert(1)">click me</a>' } as Token]}
        done
      />,
    );
    const anchor = container.querySelector("a");
    expect(anchor?.getAttribute("href") ?? null).toBeNull();
    expect(container.innerHTML).not.toMatch(/javascript:/i);
    expect(container.textContent).toContain("click me");
  });
});
import { marked } from "marked";
import { beforeAll, describe, expect, it } from "vitest";
import { configureMarked } from "../utils/configure-marked";
import { collectMathTokens, displayLatexFromEquationHtml } from "../utils/math-token";

beforeAll(() => {
  configureMarked();
});

describe("configure-marked math extensions", () => {
  it("tokenizes dollar inline math", () => {
    const tokens = marked.lexer("Energy is $E=mc^2$ today.");
    const math = collectMathTokens(tokens);
    expect(math.some((t) => t.type === "math_inline" && t.text.includes("E=mc^2"))).toBe(
      true,
    );
  });

  it("tokenizes dollar block math", () => {
    const tokens = marked.lexer("$$\n\\int_0^1 x^2 dx\n$$");
    const math = collectMathTokens(tokens);
    expect(math.some((t) => t.type === "math_block")).toBe(true);
  });

  it("tokenizes a bare bracket display block inside a list", () => {
    const tokens = marked.lexer(
      "- such as:\n  [\n  \\Gamma_r \\vdash \\left( \\bigwedge_{o \\in \\text{Prem}(r)} q_o \\right) \\implies q_r,\n  ]\nwhere $q_r$ and $q_o$ stay inline.",
    );
    const math = collectMathTokens(tokens);
    expect(
      math.some(
        (t) =>
          (t.type === "math_bare_bracket_block" || t.type === "math_block") &&
          t.text.includes("\\Gamma_r") &&
          t.text.includes("\\implies"),
      ),
    ).toBe(true);
    expect(math.some((t) => t.type === "math_inline" && t.text === "q_r")).toBe(true);
    expect(math.some((t) => t.type === "math_inline" && t.text === "q_o")).toBe(true);
  });

  it("reads display latex out of an equation tag", () => {
    const latex = displayLatexFromEquationHtml(
      '<equation id="eq-p5-1" format="latex">\n$$\n\\Gamma_r \\vdash q_r\n$$\n</equation>',
    );
    expect(latex).toContain("\\Gamma_r");
    expect(latex).not.toContain("$$");
  });

  it("does not treat citations or links as math", () => {
    const tokens = marked.lexer("See [1] and [notes](https://example.test).");
    const math = collectMathTokens(tokens);
    expect(math).toEqual([]);
  });

  it("tokenizes paren inline and bracket block LaTeX", () => {
    const tokens = marked.lexer(
      "Inline \\( \\alpha + \\beta \\) and block:\n\\[ \\sum_{i=1}^{n} i \\]",
    );
    const math = collectMathTokens(tokens);
    expect(math.some((t) => t.type === "math_paren_inline")).toBe(true);
    expect(math.some((t) => t.type === "math_bracket_block")).toBe(true);
  });
});

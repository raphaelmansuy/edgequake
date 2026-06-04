import { marked } from "marked";
import { beforeAll, describe, expect, it } from "vitest";
import { configureMarked } from "../utils/configure-marked";
import { collectMathTokens } from "../utils/math-token";

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

  it("tokenizes paren inline and bracket block LaTeX", () => {
    const tokens = marked.lexer(
      "Inline \\( \\alpha + \\beta \\) and block:\n\\[ \\sum_{i=1}^{n} i \\]",
    );
    const math = collectMathTokens(tokens);
    expect(math.some((t) => t.type === "math_paren_inline")).toBe(true);
    expect(math.some((t) => t.type === "math_bracket_block")).toBe(true);
  });
});

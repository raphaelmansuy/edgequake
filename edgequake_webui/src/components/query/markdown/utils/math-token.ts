import type { Token } from "marked";

/** Math-bearing custom token types from configure-marked extensions. */
export type MathTokenType =
  | "math_block"
  | "math_inline"
  | "math_bracket_block"
  | "math_bare_bracket_block"
  | "math_paren_inline";

const BLOCK_MATH_TYPES: ReadonlySet<MathTokenType> = new Set([
  "math_block",
  "math_bracket_block",
  "math_bare_bracket_block",
]);

const INLINE_MATH_TYPES: ReadonlySet<MathTokenType> = new Set([
  "math_inline",
  "math_paren_inline",
]);

export function isBlockMathToken(type: string): boolean {
  return BLOCK_MATH_TYPES.has(type as MathTokenType);
}

export function isInlineMathToken(type: string): boolean {
  return INLINE_MATH_TYPES.has(type as MathTokenType);
}

export interface MathTokenShape {
  type: MathTokenType;
  text: string;
  raw: string;
}

/** Extract LaTeX source from a custom math token (prefers `text` over `raw`). */
export function mathContentFromToken(token: Token): string {
  const t = token as Token & { text?: string; raw?: string };
  if (typeof t.text === "string" && t.text.trim().length > 0) {
    return t.text.trim();
  }
  const raw = typeof t.raw === "string" ? t.raw : "";
  return stripMathDelimiters(raw).trim();
}

function stripMathDelimiters(raw: string): string {
  const trimmed = raw.trim();
  if (trimmed.startsWith("$$") && trimmed.endsWith("$$")) {
    return trimmed.slice(2, -2);
  }
  if (trimmed.startsWith("$") && trimmed.endsWith("$")) {
    return trimmed.slice(1, -1);
  }
  if (trimmed.startsWith("\\(") && trimmed.endsWith("\\)")) {
    return trimmed.slice(2, -2);
  }
  if (trimmed.startsWith("\\[") && trimmed.endsWith("\\]")) {
    return trimmed.slice(2, -2);
  }
  return trimmed;
}

/** Recursively collect custom math tokens from a marked lexer result. */
export function collectMathTokens(tokens: Token[]): MathTokenShape[] {
  const out: MathTokenShape[] = [];
  for (const token of tokens) {
    if (isMathTokenType(token.type)) {
      out.push({
        type: token.type,
        text: mathContentFromToken(token),
        raw: (token as Token & { raw?: string }).raw ?? "",
      });
    }
    const childTokens = (token as Token & { tokens?: Token[] }).tokens;
    if (Array.isArray(childTokens) && childTokens.length > 0) {
      out.push(...collectMathTokens(childTokens));
    }
    const items = (token as Token & { items?: Array<{ tokens?: Token[] }> }).items;
    if (Array.isArray(items)) {
      for (const item of items) {
        if (Array.isArray(item.tokens) && item.tokens.length > 0) {
          out.push(...collectMathTokens(item.tokens));
        }
      }
    }
  }
  return out;
}

/** LaTeX body of an `<equation>` tag, or null when `raw` is not one. */
export function displayLatexFromEquationHtml(raw: string): string | null {
  const match = raw.match(/<equation\b[^>]*>([\s\S]*?)<\/equation>/i);
  if (!match) return null;
  let body = match[1].trim();
  if (body.startsWith("$$") && body.endsWith("$$")) {
    body = body.slice(2, -2).trim();
  }
  return body.length > 0 ? body : null;
}

function isMathTokenType(type: string): type is MathTokenType {
  return isBlockMathToken(type) || isInlineMathToken(type);
}
